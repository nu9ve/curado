import type { WaveData } from './waveform';

export type PhraseKind = 'intro' | 'up' | 'down' | 'chorus' | 'outro';
export interface Phrase {
  start: number;
  end: number;
  kind: PhraseKind;
}
export const PHRASE_LABELS: Record<PhraseKind, string> = {
  intro: 'INTRO',
  up: 'UP',
  down: 'DOWN',
  chorus: 'CHORUS',
  outro: 'OUTRO',
};
const BARS_PER_PHRASE = 8;

/**
 * Sections from how much bass and overall level each eight bars hold. An estimate of structure:
 * it reads energy, not harmony or vocals, so it knows nothing about a real verse.
 */
export function findPhrases(
  wave: WaveData,
  bpm: number | null,
  downbeat: number,
  duration: number,
): Phrase[] {
  if (!bpm || bpm <= 0 || duration <= 0 || !wave.count) return [];
  const span = (60 / bpm) * 4 * BARS_PER_PHRASE;
  if (duration < span * 3) return [];
  const first = downbeat - Math.ceil(downbeat / span) * span;
  const sections: { start: number; end: number; low: number; level: number }[] = [];
  for (let start = first; start < duration; start += span) {
    const from = Math.max(0, Math.floor(start * wave.rate));
    const to = Math.min(wave.count, Math.floor((start + span) * wave.rate));
    let low = 0;
    let level = 0;
    for (let bin = from; bin < to; bin++) {
      level += wave.bins[bin * 4];
      low += wave.bins[bin * 4 + 1];
    }
    const count = Math.max(1, to - from);
    sections.push({
      start: Math.max(0, start),
      end: Math.min(duration, start + span),
      low: low / count,
      level: level / count,
    });
  }
  const peakLow = Math.max(...sections.map((s) => s.low), 1);
  const peakLevel = Math.max(...sections.map((s) => s.level), 1);
  const strong = (s: { low: number; level: number }) =>
    s.low > peakLow * 0.7 && s.level > peakLevel * 0.7;
  const firstStrong = sections.findIndex(strong);
  const lastStrong = sections.length - 1 - [...sections].reverse().findIndex(strong);
  const kinds = sections.map((section, i): PhraseKind => {
    if (firstStrong >= 0 && i < firstStrong) return 'intro';
    if (firstStrong >= 0 && i > lastStrong) return 'outro';
    if (strong(section)) return 'chorus';
    const next = sections[i + 1];
    return next && strong(next) && section.level > peakLevel * 0.4 ? 'up' : 'down';
  });
  const phrases: Phrase[] = [];
  sections.forEach((section, i) => {
    const last = phrases[phrases.length - 1];
    if (last?.kind === kinds[i]) last.end = section.end;
    else phrases.push({ start: section.start, end: section.end, kind: kinds[i] });
  });
  return phrases;
}
