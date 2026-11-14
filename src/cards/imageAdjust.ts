// Non-destructive image adjustments, rendered as a CSS filter. The original
// file and its thumbnails are never modified; the values live in the card's
// data and travel with copies, exports and undo like any other edit.
import type { ImageAdjust } from '../types/models';

export type AdjustKey = keyof ImageAdjust;

export interface AdjustControl {
  key: AdjustKey;
  label: string;
  min: number;
  max: number;
  /** Value meaning "unchanged". */
  neutral: number;
  unit: '%' | '°';
}

export const ADJUST_CONTROLS: AdjustControl[] = [
  { key: 'brightness', label: 'Brightness', min: 0, max: 200, neutral: 100, unit: '%' },
  { key: 'contrast', label: 'Contrast', min: 0, max: 200, neutral: 100, unit: '%' },
  { key: 'saturation', label: 'Saturation', min: 0, max: 200, neutral: 100, unit: '%' },
  { key: 'hue', label: 'Hue', min: -180, max: 180, neutral: 0, unit: '°' },
  { key: 'grayscale', label: 'Black & white', min: 0, max: 100, neutral: 0, unit: '%' },
  { key: 'sepia', label: 'Sepia', min: 0, max: 100, neutral: 0, unit: '%' },
];

const CONTROL = Object.fromEntries(ADJUST_CONTROLS.map((c) => [c.key, c])) as Record<AdjustKey, AdjustControl>;

/** The value of `key`, clamped to its range (neutral when unset or invalid). */
export function adjustValue(adjust: ImageAdjust | undefined, key: AdjustKey): number {
  const c = CONTROL[key];
  const v = adjust?.[key];
  if (typeof v !== 'number' || !Number.isFinite(v)) return c.neutral;
  return Math.min(c.max, Math.max(c.min, v));
}

export function isNeutral(adjust: ImageAdjust | undefined): boolean {
  return ADJUST_CONTROLS.every((c) => adjustValue(adjust, c.key) === c.neutral);
}

/** CSS `filter` value, or undefined when nothing is adjusted. */
export function adjustFilter(adjust: ImageAdjust | undefined): string | undefined {
  if (!adjust || isNeutral(adjust)) return undefined;
  const v = (k: AdjustKey) => adjustValue(adjust, k);
  const parts: string[] = [];
  if (v('brightness') !== 100) parts.push(`brightness(${v('brightness') / 100})`);
  if (v('contrast') !== 100) parts.push(`contrast(${v('contrast') / 100})`);
  if (v('saturation') !== 100) parts.push(`saturate(${v('saturation') / 100})`);
  if (v('hue') !== 0) parts.push(`hue-rotate(${v('hue')}deg)`);
  if (v('grayscale') !== 0) parts.push(`grayscale(${v('grayscale') / 100})`);
  if (v('sepia') !== 0) parts.push(`sepia(${v('sepia') / 100})`);
  return parts.join(' ');
}
