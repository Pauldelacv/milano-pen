// Video/music embeds built from the URL alone — nothing is fetched.
// Ported from Milaclone's `embedUrl` (MIT, (c) 2026 ellshep).

export function isUrl(s: string): boolean {
  return /^https?:\/\/\S+$/i.test(s.trim());
}

export function embedUrl(raw: string): string | null {
  let url: URL;
  try {
    url = new URL(raw.trim());
  } catch {
    return null;
  }
  if (url.protocol !== 'https:' && url.protocol !== 'http:') return null;
  const host = url.hostname.replace(/^(www\.|m\.)/, '');
  const seg = url.pathname.split('/').filter(Boolean);
  const id = (s: string | null | undefined) => (s && /^[\w-]+$/.test(s) ? s : null);
  if (host === 'youtube.com' || host === 'youtube-nocookie.com') {
    const v = url.pathname === '/watch' ? url.searchParams.get('v') : ['shorts', 'embed', 'live'].includes(seg[0]) ? seg[1] : null;
    return id(v) ? `https://www.youtube-nocookie.com/embed/${v}` : null;
  }
  if (host === 'youtu.be') return id(seg[0]) ? `https://www.youtube-nocookie.com/embed/${seg[0]}` : null;
  if (host === 'vimeo.com' && /^\d+$/.test(seg[0] ?? '')) return `https://player.vimeo.com/video/${seg[0]}`;
  if (host === 'open.spotify.com') {
    const kinds = ['track', 'album', 'playlist', 'episode', 'show', 'artist'];
    const i = seg[0]?.startsWith('intl-') ? 1 : 0;
    if (kinds.includes(seg[i]) && id(seg[i + 1])) return `https://open.spotify.com/embed/${seg[i]}/${seg[i + 1]}`;
  }
  return null;
}

export function hostOf(raw: string): string {
  try {
    return new URL(raw).hostname.replace(/^www\./, '');
  } catch {
    return raw;
  }
}
