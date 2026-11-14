// Fonts a note can use. Three sources, none of which touch the network:
//  * fonts that ship with macOS,
//  * a few Google Fonts bundled with the app (OFL, via @fontsource — see
//    `styles/fonts.css` and THIRD_PARTY_NOTICES.md),
//  * any font installed on the Mac, by family name ("custom:<family>").

export interface FontChoice {
  id: string;
  label: string;
  /** CSS font-family stack. */
  stack: string;
  group: 'System' | 'Google Fonts';
}

const SANS = "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', Helvetica, Arial, sans-serif";
const SERIF = "ui-serif, Georgia, 'Times New Roman', serif";
const MONO = "ui-monospace, 'SF Mono', Menlo, monospace";

export const FONT_CHOICES: FontChoice[] = [
  { id: 'system', label: 'System (San Francisco)', stack: SANS, group: 'System' },
  { id: 'new-york', label: 'New York', stack: SERIF, group: 'System' },
  { id: 'rounded', label: 'SF Rounded', stack: `ui-rounded, 'SF Pro Rounded', ${SANS}`, group: 'System' },
  { id: 'mono', label: 'SF Mono', stack: MONO, group: 'System' },
  { id: 'helvetica', label: 'Helvetica Neue', stack: `'Helvetica Neue', Helvetica, Arial, sans-serif`, group: 'System' },
  { id: 'avenir', label: 'Avenir Next', stack: `'Avenir Next', Avenir, ${SANS}`, group: 'System' },
  { id: 'futura', label: 'Futura', stack: `Futura, 'Century Gothic', ${SANS}`, group: 'System' },
  { id: 'gill-sans', label: 'Gill Sans', stack: `'Gill Sans', 'Gill Sans MT', ${SANS}`, group: 'System' },
  { id: 'optima', label: 'Optima', stack: `Optima, Candara, ${SANS}`, group: 'System' },
  { id: 'georgia', label: 'Georgia', stack: `Georgia, ${SERIF}`, group: 'System' },
  { id: 'palatino', label: 'Palatino', stack: `Palatino, 'Palatino Linotype', ${SERIF}`, group: 'System' },
  { id: 'baskerville', label: 'Baskerville', stack: `Baskerville, 'Baskerville Old Face', ${SERIF}`, group: 'System' },
  { id: 'didot', label: 'Didot', stack: `Didot, 'Bodoni 72', ${SERIF}`, group: 'System' },
  { id: 'american-typewriter', label: 'American Typewriter', stack: `'American Typewriter', 'Courier New', ${MONO}`, group: 'System' },
  { id: 'menlo', label: 'Menlo', stack: `Menlo, ${MONO}`, group: 'System' },
  { id: 'noteworthy', label: 'Noteworthy', stack: `Noteworthy, 'Marker Felt', 'Comic Sans MS', cursive`, group: 'System' },
  { id: 'chalkboard', label: 'Chalkboard', stack: `'Chalkboard SE', Chalkboard, 'Comic Sans MS', cursive`, group: 'System' },
  { id: 'inter', label: 'Inter', stack: `'Inter', ${SANS}`, group: 'Google Fonts' },
  { id: 'lora', label: 'Lora', stack: `'Lora', ${SERIF}`, group: 'Google Fonts' },
  { id: 'playfair', label: 'Playfair Display', stack: `'Playfair Display', ${SERIF}`, group: 'Google Fonts' },
  { id: 'caveat', label: 'Caveat', stack: `'Caveat', cursive`, group: 'Google Fonts' },
  { id: 'jetbrains-mono', label: 'JetBrains Mono', stack: `'JetBrains Mono', ${MONO}`, group: 'Google Fonts' },
];

const BY_ID = new Map(FONT_CHOICES.map((f) => [f.id, f]));
const CUSTOM = 'custom:';

/** A family name safe to put inside a quoted CSS string, or null. */
export function cleanFamily(name: string): string | null {
  const n = name.trim().replace(/\s+/g, ' ');
  if (!n || n.length > 80 || !/^[\p{L}\p{N} ._'&+-]+$/u.test(n)) return null;
  return n;
}

export function customFontId(family: string): string | null {
  const f = cleanFamily(family);
  return f ? `${CUSTOM}${f}` : null;
}

/** The family of a custom font id, else null. */
export function customFamily(id: string | undefined): string | null {
  return id?.startsWith(CUSTOM) ? cleanFamily(id.slice(CUSTOM.length)) : null;
}

/** CSS font-family for a stored font id; undefined = the default note font. */
export function fontStack(id: string | undefined): string | undefined {
  if (!id || id === 'system') return undefined;
  const custom = customFamily(id);
  if (custom) return `"${custom.replace(/"/g, '')}", ${SANS}`;
  return BY_ID.get(id)?.stack;
}

export function fontLabel(id: string | undefined): string {
  if (!id) return BY_ID.get('system')!.label;
  return customFamily(id) ?? BY_ID.get(id)?.label ?? BY_ID.get('system')!.label;
}
