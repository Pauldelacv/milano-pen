import { describe, expect, it } from 'vitest';
import { hslToRgb, parseColor, readableInk, rgbToHex } from './colorParse';

describe('parseColor', () => {
  it('normalises hex codes', () => {
    expect(parseColor('#ff8800')).toBe('#FF8800');
    expect(parseColor('ff8800')).toBe('#FF8800');
    expect(parseColor('#f80')).toBe('#FF8800');
    expect(parseColor('  #F80F ')).toBe('#FF8800');
    expect(parseColor('#11223344')).toBe('#112233');
  });

  it('reads rgb() and hsl()', () => {
    expect(parseColor('rgb(255, 136, 0)')).toBe('#FF8800');
    expect(parseColor('rgba(255 136 0 / 0.5)')).toBe('#FF8800');
    expect(parseColor('rgb(100%, 0%, 0%)')).toBe('#FF0000');
    expect(parseColor('hsl(0, 100%, 50%)')).toBe('#FF0000');
    expect(parseColor('hsl(120 100% 25%)')).toBe('#008000');
  });

  it('rejects garbage', () => {
    expect(parseColor('')).toBeNull();
    expect(parseColor('#12')).toBeNull();
    expect(parseColor('#gggggg')).toBeNull();
    expect(parseColor('rgb(1,2)')).toBeNull();
  });
});

describe('helpers', () => {
  it('converts', () => {
    expect(rgbToHex(0, 0, 0)).toBe('#000000');
    expect(rgbToHex(300, -4, 15.6)).toBe('#FF0010');
    expect(hslToRgb(0, 0, 100)).toEqual([255, 255, 255]);
  });

  it('picks a readable ink', () => {
    expect(readableInk('#FFFFFF')).toBe('#000000');
    expect(readableInk('#111111')).toBe('#FFFFFF');
    expect(readableInk('#FFE066')).toBe('#000000');
  });
});
