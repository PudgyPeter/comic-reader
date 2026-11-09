/**
 * Math for continuous-mode resume. The reader stores where the viewport centre
 * sits inside the current page as a 0..1 fraction, so the same position survives
 * a different window size or page spacing.
 */

/** Fraction of `size` that `center` sits past `start`, clamped to 0..1. */
export function offsetWithin(center: number, start: number, size: number): number {
  if (!Number.isFinite(size) || size <= 0) return 0;
  return Math.min(1, Math.max(0, (center - start) / size));
}

/**
 * Scroll position that puts `offset` of the way into a page at the centre of the
 * viewport. Used both to restore a saved position and to keep it after images
 * finish loading and change the layout.
 */
export function scrollPositionFor(
  start: number,
  size: number,
  offset: number,
  viewport: number,
): number {
  const within = (Number.isFinite(size) ? Math.max(0, size) : 0) * clamp01(offset);
  const centering = (Number.isFinite(viewport) ? Math.max(0, viewport) : 0) / 2;
  return Math.max(0, start + within - centering);
}

export function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(1, Math.max(0, value));
}
