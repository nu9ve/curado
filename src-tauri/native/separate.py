"""Local stem separation for curado. JSON events on stdout; library logs on stderr.

Pinned adapters: demucs-mlx 1.6.1, or an installed Demucs 4 on PyTorch. Started by curado with
structured arguments; it exits when its standard input closes, or on Windows when curado's process
ends, so it cannot outlive the app.
"""
import argparse
import contextlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import struct
import sys
import threading
import traceback

PROTOCOL = sys.stdout
STEMS = ["drums", "bass", "other", "vocals", "guitar", "piano"]


def event(stage, **values):
    PROTOCOL.write(json.dumps(dict(stage=stage, **values)) + "\n")
    PROTOCOL.flush()


def probe():
    packages = {}
    for name in ["demucs", "demucs-mlx"]:
        try:
            packages[name] = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            pass
    apple = platform.system() == "Darwin" and platform.machine() == "arm64"
    print(json.dumps(dict(packages=packages, python=platform.python_version(), appleSilicon=apple)))


def write_float_wav(path, audio, rate):
    """Channels-first to IEEE float WAV, without clipping or normalizing."""
    import numpy as np
    audio = np.asarray(audio, dtype=np.float32)
    if audio.ndim != 2 or not 1 <= audio.shape[0] <= 2 or not np.isfinite(audio).all():
        raise ValueError("Invalid stem samples")
    frames, channels = audio.shape[1], audio.shape[0]
    size = frames * channels * 4
    if size > 0xFFFFFFFF - 36:
        raise ValueError("Stem exceeds WAV size limit")
    with open(path, "wb") as handle:
        handle.write(struct.pack("<4sI4s4sIHHIIHH4sI", b"RIFF", 36 + size, b"WAVE", b"fmt ", 16,
                                 3, channels, rate, rate * channels * 4, channels * 4, 32, b"data", size))
        handle.write(np.ascontiguousarray(audio.T, dtype="<f4").tobytes())


def read_float_wav(path):
    """curado's own input: IEEE float 32-bit WAV, channels first. Read here so the PyTorch path
    needs neither torchaudio's decoders nor FFmpeg (torchaudio 2.9+ moved loading to torchcodec)."""
    import numpy as np
    data = Path(path).read_bytes()
    if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        raise ValueError("Input is not a WAV file")
    fmt, samples, offset = None, None, 12
    while offset + 8 <= len(data):
        chunk, size = struct.unpack_from("<4sI", data, offset)
        body = data[offset + 8:offset + 8 + size]
        if chunk == b"fmt ":
            fmt = struct.unpack_from("<HHIIHH", body)
        elif chunk == b"data":
            samples = body
        offset += 8 + size + (size & 1)
    if fmt is None or samples is None:
        raise ValueError("Input WAV lacks fmt or data")
    kind, channels, rate, _, _, bits = fmt
    if kind not in (3, 0xFFFE) or bits != 32 or not 1 <= channels <= 2:
        raise ValueError("Input WAV must be 32-bit float, mono or stereo")
    audio = np.frombuffer(samples[:len(samples) // (4 * channels) * 4 * channels], dtype="<f4")
    return audio.reshape(-1, channels).T.copy(), rate


def demucs_mlx(args):
    import numpy as np
    from demucs_mlx import Separator
    from demucs_mlx.audio import load_audio
    separator = Separator(model=args.model, shifts=1, seed=0, batch_size=1, progress=True)
    event("separating")
    wav, _ = load_audio(args.input, sr=separator.samplerate, dtype="float32")
    wav = np.asarray(wav)
    reference = wav.mean(axis=0)
    mean = float(reference.mean())
    std = float(reference.std())
    if std < 1e-8:
        std = max(1e-8, float(wav.std()))  # Stereo anti-phase must not amplify by 1e8.
    _, stems = separator.separate_tensor((wav - mean) / std)
    return {name: audio * std + mean for name, audio in stems.items()}, separator.samplerate


def demucs_torch(args):
    import torch
    from demucs.pretrained import get_model
    from demucs.audio import convert_audio
    from demucs.apply import apply_model
    model = get_model(args.model).eval()
    audio, rate = read_float_wav(args.input)
    wav = convert_audio(torch.from_numpy(audio), rate, model.samplerate, model.audio_channels)
    # A CUDA build of PyTorch with an NVIDIA GPU separates many times faster; otherwise the CPU.
    device = args.device or ("cuda" if torch.cuda.is_available() else "cpu")
    reference = wav.mean(0)
    mean, std = reference.mean(), reference.std()
    if std < 1e-8:
        std = wav.std().clamp(min=1e-8)
    event("separating")
    with torch.no_grad():
        stems = apply_model(model, ((wav - mean) / std)[None], device=device,
                            shifts=1, split=True, overlap=0.25, progress=True, num_workers=0)[0].cpu()
    stems = stems * std + mean
    return {name: stems[i].numpy() for i, name in enumerate(model.sources)}, model.samplerate


def leave_with_parent(parent):
    if os.name == "nt" and parent:
        # Not stdin on Windows: while a thread blocks reading a synchronous pipe, any DLL whose C
        # runtime asks GetFileType about stdin waits too, and numpy's import then never returns.
        # Waiting on curado's own process works through a venv's launcher process as well.
        import ctypes
        from ctypes import wintypes
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.OpenProcess.restype = wintypes.HANDLE
        kernel32.OpenProcess.argtypes = (wintypes.DWORD, wintypes.BOOL, wintypes.DWORD)
        kernel32.WaitForSingleObject.argtypes = (wintypes.HANDLE, wintypes.DWORD)
        handle = kernel32.OpenProcess(0x00100000, False, parent)  # SYNCHRONIZE
        if not handle:
            os._exit(2)  # curado is already gone

        def watch():
            kernel32.WaitForSingleObject(handle, 0xFFFFFFFF)
            os._exit(2)
    else:
        def watch():
            # Unbuffered: a buffered read would hold a lock that aborts the interpreter at exit.
            while os.read(0, 4096):
                pass
            os._exit(2)
    threading.Thread(target=watch, daemon=True).start()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--probe", action="store_true")
    parser.add_argument("--engine", choices=["demucs-mlx", "demucs"])
    parser.add_argument("--model", choices=["htdemucs", "htdemucs_ft", "htdemucs_6s"])
    parser.add_argument("--device", choices=["cpu", "mps", "cuda"])
    parser.add_argument("--input")
    parser.add_argument("--output")
    parser.add_argument("--cache")
    parser.add_argument("--parent", type=int, help="curado's process id, watched on Windows")
    args = parser.parse_args()
    if args.probe:
        probe()
        return
    if not all([args.engine, args.model, args.input, args.output, args.cache]):
        parser.error("Missing separation arguments")
    leave_with_parent(args.parent)
    Path(args.output).mkdir(parents=True, exist_ok=True)
    Path(args.cache).mkdir(parents=True, exist_ok=True)
    os.environ["DEMUCS_MLX_CACHE_DIR"] = str(Path(args.cache) / "demucs-mlx")
    # PyTorch hub keeps models under TORCH_HOME, ~/.cache/torch by default: keep them in curado's.
    os.environ["TORCH_HOME"] = str(Path(args.cache) / "torch")
    os.environ["PYTORCH_ENABLE_MPS_FALLBACK"] = "1"
    event("loading")
    with contextlib.redirect_stdout(sys.stderr):
        stems, rate = demucs_mlx(args) if args.engine == "demucs-mlx" else demucs_torch(args)
        files = []
        event("writing")
        for name, audio in stems.items():
            if name not in STEMS:
                raise ValueError("Unexpected stem name")
            write_float_wav(Path(args.output) / (name + ".wav"), audio, int(rate))
            files.append(dict(name=name, file=name + ".wav"))
    (Path(args.output) / "result.json").write_text(
        json.dumps(dict(version=1, engine=args.engine, model=args.model, rate=int(rate), stems=files)))
    event("complete")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        traceback.print_exc(file=sys.stderr)
        event("error", message=str(error))
        sys.exit(1)
