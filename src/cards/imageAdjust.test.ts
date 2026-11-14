import { describe, expect, it } from 'vitest';
import { adjustFilter, adjustValue, isNeutral } from './imageAdjust';

describe('image adjustments', () => {
  it('produces no filter when nothing changed', () => {
    expect(adjustFilter(undefined)).toBeUndefined();
    expect(adjustFilter({})).toBeUndefined();
    expect(adjustFilter({ brightness: 100, hue: 0 })).toBeUndefined();
    expect(isNeutral({ saturation: 100 })).toBe(true);
  });

  it('builds a CSS filter in a stable order', () => {
    expect(adjustFilter({ saturation: 150, brightness: 120 })).toBe('brightness(1.2) saturate(1.5)');
    expect(adjustFilter({ hue: -90, grayscale: 100, sepia: 30, contrast: 50 })).toBe(
      'contrast(0.5) hue-rotate(-90deg) grayscale(1) sepia(0.3)',
    );
  });

  it('clamps stored values to their range', () => {
    expect(adjustValue({ brightness: 999 }, 'brightness')).toBe(200);
    expect(adjustValue({ hue: -720 }, 'hue')).toBe(-180);
    expect(adjustValue({ sepia: Number.NaN }, 'sepia')).toBe(0);
  });
});
