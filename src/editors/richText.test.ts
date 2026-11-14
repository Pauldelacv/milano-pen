import { describe, expect, it } from 'vitest';
import { docToHtml, docToText, isEmptyDoc, textToDoc } from './richText';

describe('rich text helpers', () => {
  it('converts plain text and back', () => {
    const doc = textToDoc('Hello\n\nworld');
    expect(doc.content).toHaveLength(3);
    expect(docToText(doc)).toBe('Hello\n\nworld');
    expect(isEmptyDoc(textToDoc('  '))).toBe(true);
  });

  it('renders schema HTML and drops unsafe links', () => {
    const doc = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'bold', marks: [{ type: 'bold' }] },
            { type: 'text', text: ' evil', marks: [{ type: 'link', attrs: { href: 'javascript:alert(1)' } }] },
            { type: 'text', text: ' <script>', marks: [] },
          ],
        },
      ],
    };
    const html = docToHtml(doc);
    expect(html).toContain('<strong>bold</strong>');
    expect(html).not.toContain('javascript:');
    expect(html).toContain('&lt;script&gt;');
    expect(docToHtml(doc)).toBe(html); // cached
  });

  it('flattens lists to lines', () => {
    const doc = {
      type: 'doc',
      content: [
        { type: 'bulletList', content: [
          { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'one' }] }] },
          { type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text: 'two' }] }] },
        ] },
      ],
    };
    expect(docToText(doc)).toBe('one\ntwo');
  });
});
