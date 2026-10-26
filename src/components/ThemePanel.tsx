import { useEffect, useState } from 'react';
import { Moon, Sun, Monitor, RotateCcw, Check } from 'lucide-react';
import { useTheme, updateTheme } from '../lib/theme-store';
import { COLOR_PRESETS, DEFAULT_THEME, hexToHsv, hsvToHex, normalizeHex } from '../lib/theme';
import type { ThemeMode, ThemeVariant, HSV } from '../lib/theme';
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
      <h3>Tu color. Tu ambiente.</h3>
      <p>
        El color inicial es <strong>#d66154</strong>. Los fondos, controles, waveforms y contrastes
        se adaptan al esquema que elijas.
      </p>
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
        <span className="eyebrow">CURADO</span>
        <strong>El sonido se siente distinto.</strong>
        <span>Tu biblioteca, tus controles, tu espacio.</span>
        <div>
          <button className="primary">Color principal</button>
          <kbd>Q</kbd>
          <span className="theme-meter" />
        </div>
      </div>
      <button onClick={() => updateTheme({ ...DEFAULT_THEME }, onError)}>
        <RotateCcw size={13} />
        Restaurar coral y tema inicial
      </button>
      <p className="small-print">
        Los cambios se guardan automáticamente. El color elegido se conserva; los tonos de texto se
        ajustan por separado para mantener la legibilidad. El audio no se reinicia al cambiar el
        tema.
      </p>
    </div>
  );
}
