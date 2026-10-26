import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { native } from './bridge';

/** Compact always-on-top player. Uses only the scoped window permissions in capabilities. */
export const MINI_SIZE = { width: 460, height: 132 };
export const FULL_MIN = { width: 1000, height: 700 };
let restore: { width: number; height: number } | null = null;

export async function setMiniWindow(on: boolean) {
  if (!native) return;
  const win = getCurrentWindow();
  if (on) {
    const factor = await win.scaleFactor();
    const size = (await win.innerSize()).toLogical(factor);
    restore = { width: size.width, height: size.height };
    await win.setMinSize(new LogicalSize(380, MINI_SIZE.height));
    await win.setSize(new LogicalSize(MINI_SIZE.width, MINI_SIZE.height));
    await win.setAlwaysOnTop(true);
  } else {
    await win.setAlwaysOnTop(false);
    await win.setMinSize(new LogicalSize(FULL_MIN.width, FULL_MIN.height));
    const size = restore ?? { width: 1440, height: 940 };
    await win.setSize(
      new LogicalSize(Math.max(size.width, FULL_MIN.width), Math.max(size.height, FULL_MIN.height)),
    );
  }
}
