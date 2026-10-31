import { useCallback, useState } from 'react';

export type SidePanel = 'library' | 'tools' | 'keyboard';
export interface Workspace {
  /** The sidebar is a rail of icons. Each mode remembers its own, since mixing needs the room. */
  railMix: boolean;
  railPlayer: boolean;
  panelOpen: boolean;
  panel: SidePanel;
  info: boolean;
  platters: boolean;
  autoGain: boolean;
  /** Part of the width that goes to the console when the side panel is open. */
  split: number;
}
export const DEFAULT_WORKSPACE: Workspace = {
  railMix: true,
  railPlayer: false,
  panelOpen: true,
  panel: 'library',
  info: true,
  platters: true,
  autoGain: false,
  split: 0.62,
};
export const MIN_SPLIT = 0.35;
export const MAX_SPLIT = 0.82;
const key = 'curado.workspace.v1';

export function clampSplit(split: number): number {
  if (!Number.isFinite(split)) return DEFAULT_WORKSPACE.split;
  return Math.min(MAX_SPLIT, Math.max(MIN_SPLIT, split));
}

/** Anything stored can be old or edited by hand: each field falls back on its own. */
export function parseWorkspace(text: string | null | undefined): Workspace {
  let saved: Partial<Record<keyof Workspace, unknown>> = {};
  try {
    const parsed: unknown = JSON.parse(text ?? '{}');
    if (parsed && typeof parsed === 'object') saved = parsed;
  } catch {
    return DEFAULT_WORKSPACE;
  }
  const flag = (
    name: 'railMix' | 'railPlayer' | 'panelOpen' | 'info' | 'platters' | 'autoGain',
  ) => {
    const value = saved[name];
    return typeof value === 'boolean' ? value : DEFAULT_WORKSPACE[name];
  };
  return {
    railMix: flag('railMix'),
    railPlayer: flag('railPlayer'),
    panelOpen: flag('panelOpen'),
    panel: saved.panel === 'tools' || saved.panel === 'keyboard' ? saved.panel : 'library',
    info: flag('info'),
    platters: flag('platters'),
    autoGain: flag('autoGain'),
    split: typeof saved.split === 'number' ? clampSplit(saved.split) : DEFAULT_WORKSPACE.split,
  };
}

function read(): Workspace {
  try {
    return parseWorkspace(globalThis.localStorage?.getItem(key));
  } catch {
    return DEFAULT_WORKSPACE;
  }
}

export function useWorkspace(): [Workspace, (change: Partial<Workspace>) => void] {
  const [workspace, setWorkspace] = useState(read);
  const update = useCallback((change: Partial<Workspace>) => {
    setWorkspace((current) => {
      const next = { ...current, ...change };
      try {
        globalThis.localStorage?.setItem(key, JSON.stringify(next));
      } catch {
        // A blocked or full store must not prevent using the view in this session.
      }
      return next;
    });
  }, []);
  return [workspace, update];
}
