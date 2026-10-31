/** Keep the complete console in view when practical, without making controls unusably small. */
export function consoleScale(width: number, height: number, bodyWidth: number, bodyHeight: number) {
  if (![width, height, bodyWidth, bodyHeight].every((n) => Number.isFinite(n) && n > 0)) return 1;
  const fit = Math.min(width / bodyWidth, height / bodyHeight, 1.2);
  // At very small sizes retain a usable minimum and let the existing scroll containers work.
  return Math.max(0.65, Math.floor(fit * 1000) / 1000);
}
