// What a drag-and-drop carries, read from an HTML5 DataTransfer. Drops come
// from Finder (files), from browsers (an image file, and/or its URL and HTML)
// and from other apps (text).

export interface DropContent {
  files: File[];
  /** Web addresses (from text/uri-list, or text/plain that is a URL). */
  urls: string[];
  /** Images embedded as data: URLs in dropped HTML (no network needed). */
  dataImages: string[];
  text: string;
}

/** The subset of DataTransfer we read (easy to fake in tests). */
export interface DataTransferLike {
  files?: ArrayLike<File> | null;
  types?: readonly string[] | ArrayLike<string>;
  getData(type: string): string;
}

const URL_RE = /^https?:\/\/\S+$/i;

export function parseUriList(list: string): string[] {
  return list
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l && !l.startsWith('#') && URL_RE.test(l));
}

/** `src` of <img> tags whose source is an inline data: image. */
export function dataImagesInHtml(html: string): string[] {
  const out: string[] = [];
  const re = /<img\b[^>]*?\bsrc\s*=\s*(["'])(data:image\/[a-z0-9.+-]+;base64,[a-z0-9+/=\s]+)\1/gi;
  let m: RegExpExecArray | null;
  while ((m = re.exec(html))) out.push(m[2].replace(/\s+/g, ''));
  return out;
}

export function readDrop(dt: DataTransferLike): DropContent {
  const files = dt.files ? Array.from(dt.files) : [];
  const types = dt.types ? Array.from(dt.types) : [];
  const has = (t: string) => types.includes(t);
  const text = has('text/plain') ? dt.getData('text/plain') : '';
  let urls = has('text/uri-list') ? parseUriList(dt.getData('text/uri-list')) : [];
  if (!urls.length && URL_RE.test(text.trim())) urls = [text.trim()];
  const dataImages = !files.length && has('text/html') ? dataImagesInHtml(dt.getData('text/html')) : [];
  return { files, urls, dataImages, text };
}

/** Decode a data: URL into a File (for images dropped as inline HTML). */
export function dataUrlToFile(url: string, name = 'dropped'): File | null {
  const m = /^data:(image\/[a-z0-9.+-]+);base64,(.*)$/i.exec(url);
  if (!m) return null;
  const ext = m[1].split('/')[1].replace('jpeg', 'jpg').replace('svg+xml', 'svg').replace(/[^a-z0-9]/gi, '');
  try {
    const bin = atob(m[2]);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return new File([bytes], `${name}.${ext || 'png'}`, { type: m[1] });
  } catch {
    return null;
  }
}

/** True when the drag carries something Milano Pen can turn into cards. */
export function isUsefulDrag(types: readonly string[] | ArrayLike<string> | undefined): boolean {
  const t = types ? Array.from(types) : [];
  return t.some((x) => x === 'Files' || x === 'text/uri-list' || x === 'text/plain' || x === 'text/html');
}
