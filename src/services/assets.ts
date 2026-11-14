// URLs of imported files, served read-only by the Rust `milano://` protocol.

export type AssetVariant = '256' | '1024' | 'original';

const isWindows = typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent);
const BASE = isWindows ? 'http://milano.localhost' : 'milano://localhost';

export function assetUrl(assetId: string, variant: AssetVariant): string {
  return `${BASE}/asset/${assetId}/${variant}`;
}

/** The smallest variant that stays sharp at `screenPixels` on-screen width. */
export function pickVariant(screenPixels: number): AssetVariant {
  if (screenPixels <= 256) return '256';
  if (screenPixels <= 1024) return '1024';
  return 'original';
}

export function formatBytes(n: number | undefined): string {
  if (n == null || !Number.isFinite(n)) return '';
  const units = ['B', 'KB', 'MB', 'GB'];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v < 10 && i > 0 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}
