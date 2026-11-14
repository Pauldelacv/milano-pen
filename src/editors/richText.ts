// Shared Tiptap configuration and helpers. Notes store Tiptap JSON in
// `data.doc` and its plain text in `data.text` (the same contract as
// Milaclone, so its exports and importer shapes stay compatible).
import { generateHTML, type JSONContent } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';

export const extensions = [
  StarterKit.configure({
    heading: { levels: [1, 2, 3] },
    link: {
      openOnClick: false,
      autolink: true,
      protocols: ['mailto'],
      isAllowedUri: (url, ctx) => /^(https?:|mailto:)/i.test(url) && ctx.defaultValidate(url),
      HTMLAttributes: { rel: 'noopener noreferrer nofollow', target: null },
    },
  }),
];

/** A document of one paragraph per line. */
export function textToDoc(text: string): JSONContent {
  const lines = text.split('\n');
  return {
    type: 'doc',
    content: lines.map((line) => (line ? { type: 'paragraph', content: [{ type: 'text', text: line }] } : { type: 'paragraph' })),
  };
}

/** Plain text, one block per line (what Tiptap's getText would produce). */
export function docToText(node: JSONContent | undefined): string {
  if (!node) return '';
  if (node.type === 'text') return node.text ?? '';
  if (node.type === 'hardBreak') return '\n';
  const parts = (node.content ?? []).map(docToText);
  const blocky = ['doc', 'bulletList', 'orderedList', 'blockquote', 'listItem'];
  return parts.join(blocky.includes(node.type ?? '') ? '\n' : '');
}

export function isEmptyDoc(doc: JSONContent | undefined): boolean {
  return !docToText(doc).trim();
}

// Static rendering of non-edited notes: HTML generated from the schema (never
// from stored HTML), cached per document object.
const htmlCache = new WeakMap<object, string>();

export function docToHtml(doc: JSONContent): string {
  const hit = htmlCache.get(doc);
  if (hit !== undefined) return hit;
  let html: string;
  try {
    html = generateHTML(doc, extensions);
  } catch {
    html = '';
  }
  htmlCache.set(doc, html);
  return html;
}

export function noteDoc(data: { doc?: JSONContent; text?: string }): JSONContent {
  return data.doc ?? textToDoc(data.text ?? '');
}
