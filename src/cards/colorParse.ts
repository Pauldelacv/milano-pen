// Colour input for swatch cards: hex (#rgb, #rgba, #rrggbb, #rrggbbaa, with or
// without "#"), rgb()/rgba(), hsl()/hsla(), and — in the browser — any CSS
// colour name. Everything is normalised to upper-case "#RRGGBB".

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
const hex2 = (n: number) => Math.round(clamp(n, 0, 255)).toString(16).padStart(2, '0');

export function rgbToHex(r: number, g: number, b: number): string {
  return `#${hex2(r)}${hex2(g)}${hex2(b)}`.toUpperCase();
}

export function hexToRgb(hex: string): [number, number, number] | null {
  const m = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!m) return null;
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** h in degrees, s and l in 0–100. */
export function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const hh = (((h % 360) + 360) % 360) / 360;
  const ss = clamp(s, 0, 100) / 100;
  const ll = clamp(l, 0, 100) / 100;
  if (ss === 0) return [ll * 255, ll * 255, ll * 255];
  const q = ll < 0.5 ? ll * (1 + ss) : ll + ss - ll * ss;
  const p = 2 * ll - q;
  const channel = (t: number) => {
    const tt = t < 0 ? t + 1 : t > 1 ? t - 1 : t;
    if (tt < 1 / 6) return p + (q - p) * 6 * tt;
    if (tt < 1 / 2) return q;
    if (tt < 2 / 3) return p + (q - p) * (2 / 3 - tt) * 6;
    return p;
  };
  return [channel(hh + 1 / 3) * 255, channel(hh) * 255, channel(hh - 1 / 3) * 255];
}

function parseHex(s: string): string | null {
  const m = /^#?([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.exec(s);
  if (!m) return null;
  let h = m[1];
  if (h.length <= 4) h = [...h.slice(0, 3)].map((c) => c + c).join('');
  return `#${h.slice(0, 6)}`.toUpperCase();
}

/** "12", "12%" (of 255) — rgb() channel. */
function channel(v: string): number | null {
  const n = parseFloat(v);
  if (!Number.isFinite(n)) return null;
  return v.trim().endsWith('%') ? (n / 100) * 255 : n;
}

function parseFunctional(s: string): string | null {
  const m = /^(rgba?|hsla?)\(\s*([^)]*)\)$/i.exec(s);
  if (!m) return null;
  // Accept both "1, 2, 3" and "1 2 3 / 0.5".
  const parts = m[2].split(/[\s,/]+/).filter(Boolean);
  if (parts.length < 3) return null;
  if (m[1].toLowerCase().startsWith('rgb')) {
    const [r, g, b] = parts.slice(0, 3).map(channel);
    if (r == null || g == null || b == null) return null;
    return rgbToHex(r, g, b);
  }
  const h = parseFloat(parts[0]);
  const sat = parseFloat(parts[1]);
  const l = parseFloat(parts[2]);
  if (![h, sat, l].every(Number.isFinite)) return null;
  return rgbToHex(...hslToRgb(h, sat, l));
}

/** Resolve CSS colour names through the browser's own parser. */
function parseNamed(s: string): string | null {
  if (!/^[a-z]+$/i.test(s) || typeof document === 'undefined') return null;
  let ctx: CanvasRenderingContext2D | null = null;
  try {
    ctx = document.createElement('canvas').getContext('2d');
  } catch {
    return null;
  }
  if (!ctx) return null;
  // Two sentinels: an unknown name leaves the previous value in place.
  ctx.fillStyle = '#000000';
  ctx.fillStyle = s;
  const a = ctx.fillStyle;
  ctx.fillStyle = '#ffffff';
  ctx.fillStyle = s;
  const b = ctx.fillStyle;
  if (a !== b || typeof a !== 'string') return null;
  return parseHex(a) ?? parseFunctional(a);
}

/** Normalised "#RRGGBB", or null if `input` isn't a colour. */
export function parseColor(input: string): string | null {
  const s = input.trim();
  if (!s) return null;
  return parseHex(s) ?? parseFunctional(s) ?? parseNamed(s);
}

/** Black or white, whichever reads better on `hex`. */
export function readableInk(hex: string): '#000000' | '#FFFFFF' {
  const rgb = hexToRgb(hex);
  if (!rgb) return '#000000';
  const [r, g, b] = rgb.map((c) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  });
  const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  return lum > 0.4 ? '#000000' : '#FFFFFF';
}
