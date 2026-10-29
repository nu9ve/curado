import { tr } from '../lib/language';
import { useTheme, currentPalette } from '../lib/theme-store';
import { rgb } from '../lib/theme';
import { useEffect, useRef, useState } from 'react';
import { Maximize2, Eye, EyeOff } from 'lucide-react';
import { call, getSnapshot } from '../lib/bridge';
export function Visual({ onError }: { onError: (e: unknown) => void }) {
  const theme = useTheme();
  const ref = useRef<HTMLCanvasElement>(null);
  const [enabled, setEnabled] = useState(true);
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas || !enabled) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    let id = 0,
      last = 0;
    let w = 0,
      h = 0;
    const resize = () => {
      const dpr = Math.min(devicePixelRatio, 1.5);
      w = canvas.clientWidth;
      h = canvas.clientHeight;
      canvas.width = w * dpr;
      canvas.height = h * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(canvas);
    const colors = currentPalette();
    const color = rgb(colors.accent).map((c) => Math.round(c * 255));
    const draw = (now: number) => {
      id = requestAnimationFrame(draw);
      if (document.hidden || now - last < 33) return;
      last = now;
      ctx.fillStyle = colors.tokens['--visual-bg'];
      ctx.fillRect(0, 0, w, h);
      const audio = getSnapshot().audio;
      const energy = audio?.rms ?? 0;
      const wave = audio?.waveform ?? [];
      const t = now / 1800;
      for (let row = 27; row >= 0; row--) {
        ctx.beginPath();
        ctx.strokeStyle = `rgba(${color.join(',')},${0.08 + (1 - row / 28) * 0.5})`;
        ctx.lineWidth = 0.7;
        for (let col = 0; col <= 48; col++) {
          const x = (col / 48 - 0.5) * 14;
          const z = 3 + row * 0.65;
          const y =
            Math.sin(x * 0.8 + row * 0.35 + t) *
              Math.cos(row * 0.3 - t * 0.4) *
              (0.35 + energy * 4) +
            Math.abs(wave[col % 64] ?? 0) * 2;
          const sx = w / 2 + (x / z) * w * 0.45;
          const sy = h * 0.27 + ((2.3 - y) / z) * h * 0.68;
          if (col === 0) ctx.moveTo(sx, sy);
          else ctx.lineTo(sx, sy);
        }
        ctx.stroke();
      }
    };
    id = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(id);
      ro.disconnect();
    };
  }, [enabled, theme]);
  return (
    <section className="visual">
      <canvas ref={ref} />
      <div className="visual-overlay">
        <span className="eyebrow">{tr('paisaje sonoro', 'soundscape')}</span>
        <h2>
          {tr('deja que la música', 'let the music')} <br />
          {tr('dibuje el espacio', 'draw the space')}{' '}
        </h2>
        <p>{tr('procedural · reactivo al master', 'procedural · reacts to master')}</p>
      </div>
      <div className="visual-controls">
        <button
          onClick={() => setEnabled((v) => !v)}
          title={
            enabled
              ? tr('Pausar visual integrado', 'Pause integrated visual')
              : tr('Activar visual integrado', 'Enable integrated visual')
          }
        >
          {enabled ? <Eye size={14} /> : <EyeOff size={14} />}
        </button>
        {document.documentElement.dataset.platform === 'macos' && (
          <button onClick={() => call('open_visuals').catch(onError)}>
            <Maximize2 size={13} />
            Metal
          </button>
        )}
        <span>30 fps</span>
      </div>
    </section>
  );
}
