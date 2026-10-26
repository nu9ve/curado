import { useTheme, currentPalette } from '../lib/theme-store';
import { useEffect, useRef } from 'react';
interface Props {
  peaks: number[];
  progress: number;
  color?: string;
  onSeek?: (fraction: number) => void;
  compact?: boolean;
  label?: string;
}
export function Waveform({
  peaks,
  progress,
  color,
  onSeek,
  compact = false,
  label = 'Posición de reproducción',
}: Props) {
  const theme = useTheme();
  const themeColor = color ?? currentPalette().accent;
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const draw = () => {
      const dpr = Math.min(window.devicePixelRatio, 2);
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      canvas.width = width * dpr;
      canvas.height = height * dpr;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.scale(dpr, dpr);
      ctx.clearRect(0, 0, width, height);
      ctx.strokeStyle = currentPalette().dark ? '#ffffff0c' : '#0000000c';
      ctx.lineWidth = 1;
      for (let x = 0; x < width; x += width / 8) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();
      }
      const bars = Math.floor(width / 3);
      const center = height / 2;
      for (let i = 0; i < bars; i++) {
        const p = peaks[Math.floor((i / bars) * peaks.length)] ?? 0;
        const h = Math.max(2, Math.min(1, p) * height * 0.85);
        ctx.fillStyle = i / bars < progress ? themeColor : `${themeColor}55`;
        ctx.fillRect(i * 3, center - h / 2, 2, h);
      }
      if (!compact) {
        ctx.strokeStyle = currentPalette().text;
        ctx.beginPath();
        ctx.moveTo(progress * width, 0);
        ctx.lineTo(progress * width, height);
        ctx.stroke();
        ctx.fillStyle = currentPalette().text;
        ctx.fillRect(progress * width - 3, 0, 6, 5);
      }
    };
    draw();
    const observer = new ResizeObserver(draw);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [peaks, progress, themeColor, compact, theme]);
  return (
    <canvas
      ref={ref}
      className={`waveform ${compact ? 'compact' : ''}`}
      role={onSeek ? 'slider' : 'img'}
      aria-label={label}
      aria-valuemin={onSeek ? 0 : undefined}
      aria-valuemax={onSeek ? 100 : undefined}
      aria-valuenow={onSeek ? Math.round(progress * 100) : undefined}
      tabIndex={onSeek ? 0 : undefined}
      onClick={(e) =>
        onSeek?.(
          (e.clientX - e.currentTarget.getBoundingClientRect().left) / e.currentTarget.clientWidth,
        )
      }
      onKeyDown={(e) => {
        if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
          e.preventDefault();
          e.stopPropagation();
          onSeek?.(Math.max(0, Math.min(1, progress + (e.key === 'ArrowRight' ? 0.01 : -0.01))));
        }
      }}
    />
  );
}
