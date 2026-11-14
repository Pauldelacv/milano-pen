import { describe, expect, it } from 'vitest';
import { embedUrl, hostOf, isUrl } from './embeds';

describe('embeds', () => {
  it('maps known providers', () => {
    expect(embedUrl('https://www.youtube.com/watch?v=dQw4w9WgXcQ')).toBe('https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ');
    expect(embedUrl('https://youtu.be/abc_DEF-1')).toBe('https://www.youtube-nocookie.com/embed/abc_DEF-1');
    expect(embedUrl('https://youtube.com/shorts/xyz')).toBe('https://www.youtube-nocookie.com/embed/xyz');
    expect(embedUrl('https://vimeo.com/12345')).toBe('https://player.vimeo.com/video/12345');
    expect(embedUrl('https://open.spotify.com/intl-de/track/4uLU6hMCjMI75M1A2tKUQC')).toBe(
      'https://open.spotify.com/embed/track/4uLU6hMCjMI75M1A2tKUQC',
    );
  });

  it('rejects everything else', () => {
    expect(embedUrl('https://example.com/watch?v=x')).toBeNull();
    expect(embedUrl('https://www.youtube.com/watch?v=<script>')).toBeNull();
    expect(embedUrl('javascript:alert(1)')).toBeNull();
    expect(embedUrl('not a url')).toBeNull();
  });

  it('detects urls and hosts', () => {
    expect(isUrl(' https://a.b/c ')).toBe(true);
    expect(isUrl('a b')).toBe(false);
    expect(hostOf('https://www.example.com/x')).toBe('example.com');
  });
});
