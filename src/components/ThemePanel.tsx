import { tr } from '../lib/language';
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
const styleEnglish: Record<string, [string, string]> = {
  curado: ['curado', 'Warm and forward-looking: soft gradients and pixel details.'],
  pixel: ['pixel', 'Hard edges, blocks and dither: an 8-bit console.'],
  gothic: ['gothic', 'Deep black, crimson and gothic type for late sessions.'],
  y2k: ['y2k', 'Chrome, shine and bubbles: the future from twenty-five years ago.'],
  minimal: ['minimal', 'Flat and quiet. No textures, just essential color.'],
};
const colorEnglish: Record<string, string> = {
  Ámbar: 'Amber',
  Salvia: 'Sage',
  Océano: 'Ocean',
  Lavanda: 'Lavender',
  Rosa: 'Pink',
  Coral: 'Coral',
};
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
    } else
      onError(
        new Error(
          tr(
            'Usa un color hexadecimal como #d66154 o #d65.',
            'Use a hexadecimal color such as #d66154 or #d65.',
          ),
        ),
      );
  };
  return (
    <div className="settings-body theme-panel">
      <h3>{tr('tu color. tu ambiente.', 'your color. your atmosphere.')}</h3>
      <p>
        {tr(
          'Elige un estilo y después ajusta su color: cada estilo define materiales, tipografía y texturas; el color base (inicial',
          'Choose a style and adjust its color: each style defines materials, typography and textures; the base color (initially',
        )}{' '}
        <strong>#d66154</strong>
        {tr(') sigue siendo tuyo.', ') remains yours.')}{' '}
      </p>
      <span className="eyebrow">{tr('ESTILO', 'STYLE')}</span>
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
              <strong>{tr(STYLES[id].name, styleEnglish[id][0])}</strong>
              <small>{tr(STYLES[id].description, styleEnglish[id][1])}</small>
            </button>
          );
        })}
      </div>
      {theme.base !== STYLES[theme.style].suggested && (
        <button className="subtle" onClick={() => patch({ base: STYLES[theme.style].suggested })}>
          {tr('Usar el color sugerido de', 'Use suggested color for')} {STYLES[theme.style].name} ·{' '}
          {STYLES[theme.style].suggested}
        </button>
      )}
      <span className="eyebrow">{tr('APARIENCIA', 'APPEARANCE')}</span>
      <div className="theme-modes">
        {(
          [
            ['dark', tr('Oscuro', 'Dark'), Moon],
            ['light', tr('Claro', 'Light'), Sun],
            ['system', tr('Sistema', 'System'), Monitor],
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
        {tr('Variación', 'Variation')}{' '}
        <select
          value={theme.variant}
          onChange={(e) => patch({ variant: e.target.value as ThemeVariant })}
        >
          <option value="neutral">
            {tr('Neutro · color en los acentos', 'Neutral · color in accents')}
          </option>
          <option value="tinted">
            {tr('Tonal · superficies teñidas', 'Tinted · colored surfaces')}
          </option>
          <option value="contrast">
            {tr(
              'Contraste alto · bordes y texto reforzados',
              'High contrast · stronger edges and text',
            )}
          </option>
        </select>
      </label>
      <div className="theme-selects">
        <label>
          {tr('Tipografía', 'Typography')}{' '}
          <select
            value={theme.typography}
            onChange={(e) => patch({ typography: e.target.value as Typography })}
          >
            <option value="auto">
              {tr('Del estilo ·', 'From style ·')}{' '}
              {resolved({ ...theme, typography: 'auto' }).typography}
            </option>
            <option value="display">
              {tr('Display · Jersey 15 + sistema', 'Display · Jersey 15 + system')}
            </option>
            <option value="pixel">Pixel · Pixelify Sans</option>
            <option value="mono">Mono · Geist Mono</option>
            <option value="system">
              {tr('Sistema · nativa del OS', 'System · native OS font')}
            </option>
          </select>
        </label>
        <label>
          {tr('Textura', 'Texture')}{' '}
          <select
            value={theme.texture}
            onChange={(e) => patch({ texture: e.target.value as Texture })}
          >
            <option value="auto">
              {tr('Del estilo ·', 'From style ·')} {resolved({ ...theme, texture: 'auto' }).texture}
            </option>
            <option value="dither">
              {tr('Dither · pixeles ordenados', 'Dither · ordered pixels')}
            </option>
            <option value="smooth">{tr('Gradientes suaves', 'Soft gradients')}</option>
            <option value="flat">{tr('Plano', 'Flat')}</option>
          </select>
        </label>
        <label>
          Waveform
          <select
            value={theme.waveform}
            onChange={(e) => patch({ waveform: e.target.value as WaveStyle })}
          >
            <option value="bands">
              {tr('3 bandas · graves, medios, agudos', '3 bands · low, mid, high')}
            </option>
            <option value="rgb">
              {tr('RGB · color por frecuencia', 'RGB · color by frequency')}
            </option>
            <option value="mono">{tr('Mono · color principal', 'Mono · accent color')}</option>
          </select>
        </label>
      </div>
      <div className="theme-color-heading">
        <span className="eyebrow">{tr('COLOR PRINCIPAL', 'ACCENT COLOR')}</span>
        <span className="theme-current-code">{theme.base}</span>
      </div>
      <div className="color-presets">
        {COLOR_PRESETS.map((p) => (
          <button
            key={p.value}
            title={tr(p.name, colorEnglish[p.name] ?? p.name)}
            aria-label={`Color ${tr(p.name, colorEnglish[p.name] ?? p.name)} ${p.value}`}
            aria-pressed={theme.base === p.value}
            style={{ backgroundColor: p.value, color: '#111' }}
            onClick={() => patch({ base: p.value })}
          >
            {theme.base === p.value && <Check size={16} />}
            <span>{tr(p.name, colorEnglish[p.name] ?? p.name)}</span>
          </button>
        ))}
      </div>
      <div className="custom-color">
        <label>
          {tr('Selector', 'Picker')}{' '}
          <input
            aria-label={tr('Elegir color principal', 'Choose accent color')}
            type="color"
            value={theme.base}
            onChange={(e) => patch({ base: e.target.value })}
          />
        </label>
        <label>
          Hexadecimal
          <input
            aria-label={tr('Color hexadecimal', 'Hex color')}
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
        <button onClick={commitHex}>{tr('Aplicar', 'Apply')}</button>
      </div>
      <div className="hsv-controls">
        <Fader
          label={tr('Tono H', 'Hue H')}
          min={0}
          max={359}
          step={1}
          value={hsv.h}
          format={(v) => `${Math.round(v)}°`}
          onChange={(h) => setHsv({ h })}
        />
        <Fader
          label={tr('Saturación S', 'Saturation S')}
          value={hsv.s}
          onChange={(s) => setHsv({ s })}
        />
        <Fader
          label={tr('Brillo V', 'Brightness V')}
          value={hsv.v}
          onChange={(v) => setHsv({ v })}
        />
      </div>
      <div
        className="theme-preview"
        aria-label={tr('Muestra del tema actual', 'Current theme preview')}
      >
        <span className="eyebrow">curado</span>
        <strong>{tr('el sonido se siente distinto.', 'sound feels different.')}</strong>
        <span>
          {tr(
            'Tu biblioteca, tus controles, tu espacio.',
            'Your library, your controls, your space.',
          )}
        </span>
        <div>
          <button className="primary">{tr('Color principal', 'Accent color')}</button>
          <kbd>Q</kbd>
          <span className="theme-meter" />
        </div>
      </div>
      <button onClick={() => updateTheme({ ...DEFAULT_THEME }, onError)}>
        <RotateCcw size={13} />
        {tr(
          'Restaurar estilo curado y coral inicial',
          'Restore curado style and original coral',
        )}{' '}
      </button>
      <p className="small-print">
        {tr(
          'Los cambios se guardan automáticamente. El color elegido se conserva; los tonos de texto se ajustan por separado para mantener la legibilidad. El audio no se reinicia al cambiar el tema.',
          'Changes save automatically. Your color is preserved; text colors adjust independently for readability. Audio does not restart when the theme changes.',
        )}{' '}
      </p>
    </div>
  );
}
