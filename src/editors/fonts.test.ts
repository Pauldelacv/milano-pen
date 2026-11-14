import { describe, expect, it } from 'vitest';
import { cleanFamily, customFamily, customFontId, fontLabel, fontStack } from './fonts';

describe('fonts', () => {
  it('maps built-in ids to stacks', () => {
    expect(fontStack(undefined)).toBeUndefined();
    expect(fontStack('system')).toBeUndefined();
    expect(fontStack('lora')).toMatch(/^'Lora', /);
    expect(fontStack('unknown')).toBeUndefined();
    expect(fontLabel('caveat')).toBe('Caveat');
  });

  it('accepts installed fonts by family name', () => {
    const id = customFontId('  Source   Sans 3 ');
    expect(id).toBe('custom:Source Sans 3');
    expect(customFamily(id!)).toBe('Source Sans 3');
    expect(fontStack(id!)).toMatch(/^"Source Sans 3", /);
    expect(fontLabel(id!)).toBe('Source Sans 3');
  });

  it('refuses names that could break out of CSS', () => {
    expect(cleanFamily('x"; color: red')).toBeNull();
    expect(cleanFamily('a{b}')).toBeNull();
    expect(cleanFamily('')).toBeNull();
    expect(customFontId('url(evil)')).toBeNull();
    expect(fontStack('custom:bad;name')).toBeUndefined();
  });
});
