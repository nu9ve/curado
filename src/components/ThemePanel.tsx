import { useEffect, useState } from 'react';
import { Moon, Sun, Monitor, RotateCcw, Check } from 'lucide-react';
import { useTheme, updateTheme } from '../lib/theme-store';
import {
  COLOR_PRESETS,
  DEFAULT_THEME,
  STYLES,
  STYLE_IDS,
  hexToHsv,
  hsvToHex,
  normalizeHex,
  palette,
  resolved,
} from '../lib/theme';
import type { ThemeMode, ThemeVariant, HSV, Typography, Texture, WaveStyle } from '../lib/theme';
import { Fader } from './Controls';
export function ThemePanel({ onError }: { onError: (e: unknown) => void }) {
  const theme = useTheme();
  const [hsv, setHsvState] = useState(() => hexToHsv(theme.base));
  useEffect(() => {
    setHsvState((previous) =>
      hsvToHex(previous) === theme.base ? previous : hexToHsv(theme.base),
    );
  }, [theme.base]);
  const [draft, setDraft] = useState(theme.base);
  const [editing, setEditing] = useState(false);
  const patch = (value: Partial<typeof theme>) => updateTheme({ ...theme, ...value }, onError);
  const setHsv = (value: Partial<HSV>) => {
    const next = { ...hsv, ...value };
    setHsvState(next);
    patch({ base: hsvToHex(next) });
  };
  const commitHex = () => {
    const hex = normalizeHex(editing ? draft : theme.base);
    if (hex) {
      patch({ base: hex });
      setDraft(hex);
      setEditing(false);
    } else onError(new Error('Usa un color hexadecimal como #d66154 o #d65.'));
  };
  return (
    <div className="settings-body theme-panel">
      <h3>tu color. tu ambiente.</h3>
      <p>
        Elige un estilo y después ajusta su color: cada estilo define materiales, tipografía y
        texturas; el color base (inicial <strong>#d66154</strong>) sigue siendo tuyo.
      </p>
      <span className="eyebrow">ESTILO</span>
      <div className="style-grid">
        {STYLE_IDS.map((id) => {
          const preview = palette({ ...theme, style: id }, theme.mode !== 'light');
          return (
            <button
              key={id}
              className={`style-card ${theme.style === id ? 'active' : ''}`}
              aria-pressed={theme.style === id}
              data-style-preview={id}
              onClick={() => patch({ style: id })}
              style={{ background: preview.chrome, color: preview.text }}
            >
              <i
                style={{
                  background: `linear-gradient(135deg, ${preview.accent}, ${preview.gradient})`,
                }}
              />
              <strong>{STYLES[id].name}</strong>
              <small>{STYLES[id].description}</small>
            </button>
          );
        })}
      </div>
      {theme.base !== STYLES[theme.style].suggested && (
        <button className="subtle" onClick={() => patch({ base: STYLES[theme.style].suggested })}>
          Usar el color sugerido de {STYLES[theme.style].name} · {STYLES[theme.style].suggested}
        </button>
      )}
      <span className="eyebrow">APARIENCIA</span>
      <div className="theme-modes">
        {(
          [
            ['dark', 'Oscuro', Moon],
            ['light', 'Claro', Sun],
            ['system', 'Sistema', Monitor],
          ] as const
        ).map(([mode, label, Icon]) => (
          <button
            key={mode}
            className={theme.mode === mode ? 'active' : ''}
            aria-pressed={theme.mode === mode}
            onClick={() => patch({ mode: mode as ThemeMode })}
          >
            <Icon size={18} />
            {label}
          </button>
        ))}
      </div>
      <label>
        Variación
        <select
          value={theme.variant}
          onChange={(e) => patch({ variant: e.target.value as ThemeVariant })}
        >
          <option value="neutral">Neutro · color en los acentos</option>
          <option value="tinted">Tonal · superficies teñidas</option>
          <option value="contrast">Contraste alto · bordes y texto reforzados</option>
        </select>
      </label>
      <div className="theme-selects">
        <label>
          Tipografía
          <select
            value={theme.typography}
            onChange={(e) => patch({ typography: e.target.value as Typography })}
          >
            <option value="auto">
              Del estilo · {resolved({ ...theme, typography: 'auto' }).typography}
            </option>
            <option value="display">Display · Jersey 15 + sistema</option>
            <option value="pixel">Pixel · Pixelify Sans</option>
            <option value="mono">Mono · Geist Mono</option>
            <option value="system">Sistema · nativa del OS</option>
          </select>
        </label>
        <label>
          Textura
          <select
            value={theme.texture}
            onChange={(e) => patch({ texture: e.target.value as Texture })}
          >
            <option value="auto">
              Del estilo · {resolved({ ...theme, texture: 'auto' }).texture}
            </option>
            <option value="dither">Dither · pixeles ordenados</option>
            <option value="smooth">Gradientes suaves</option>
            <option value="flat">Plano</option>
          </select>
        </label>
        <label>
          Waveform
          <select
            value={theme.waveform}
            onChange={(e) => patch({ waveform: e.target.value as WaveStyle })}
          >
            <option value="bands">3 bandas · graves, medios, agudos</option>
            <option value="rgb">RGB · color por frecuencia</option>
            <option value="mono">Mono · color principal</option>
          </select>
        </label>
      </div>
      <div className="theme-color-heading">
        <span className="eyebrow">COLOR PRINCIPAL</span>
        <span className="theme-current-code">{theme.base}</span>
      </div>
      <div className="color-presets">
        {COLOR_PRESETS.map((p) => (
          <button
            key={p.value}
            title={p.name}
            aria-label={`Color ${p.name} ${p.value}`}
            aria-pressed={theme.base === p.value}
            style={{ backgroundColor: p.value, color: '#111' }}
            onClick={() => patch({ base: p.value })}
          >
            {theme.base === p.value && <Check size={16} />}
            <span>{p.name}</span>
          </button>
        ))}
      </div>
      <div className="custom-color">
        <label>
          Selector
          <input
            aria-label="Elegir color principal"
            type="color"
            value={theme.base}
            onChange={(e) => patch({ base: e.target.value })}
          />
        </label>
        <label>
          Hexadecimal
          <input
            aria-label="Color hexadecimal"
            value={editing ? draft : theme.base}
            onFocus={() => {
              setDraft(theme.base);
              setEditing(true);
            }}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commitHex}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                commitHex();
              }
            }}
            maxLength={7}
          />
        </label>
        <button onClick={commitHex}>Aplicar</button>
      </div>
      <div className="hsv-controls">
        <Fader
          label="Tono H"
          min={0}
          max={359}
          step={1}
          value={hsv.h}
          format={(v) => `${Math.round(v)}°`}
          onChange={(h) => setHsv({ h })}
        />
        <Fader label="Saturación S" value={hsv.s} onChange={(s) => setHsv({ s })} />
        <Fader label="Brillo V" value={hsv.v} onChange={(v) => setHsv({ v })} />
      </div>
      <div className="theme-preview" aria-label="Muestra del tema actual">
        <span className="eyebrow">curado</span>
        <strong>el sonido se siente distinto.</strong>
        <span>Tu biblioteca, tus controles, tu espacio.</span>
        <div>
          <button className="primary">Color principal</button>
          <kbd>Q</kbd>
          <span className="theme-meter" />
        </div>
      </div>
      <button onClick={() => updateTheme({ ...DEFAULT_THEME }, onError)}>
        <RotateCcw size={13} />
        Restaurar estilo curado y coral inicial
      </button>
      <p className="small-print">
        Los cambios se guardan automáticamente. El color elegido se conserva; los tonos de texto se
        ajustan por separado para mantener la legibilidad. El audio no se reinicia al cambiar el
        tema.
      </p>
    </div>
  );
}
