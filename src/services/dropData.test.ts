import { describe, expect, it } from 'vitest';
import { dataImagesInHtml, dataUrlToFile, isUsefulDrag, parseUriList, readDrop, type DataTransferLike } from './dropData';

function dt(data: Record<string, string>, files: File[] = []): DataTransferLike {
  return {
    files,
    types: [...(files.length ? ['Files'] : []), ...Object.keys(data)],
    getData: (t) => data[t] ?? '',
  };
}

describe('drop data', () => {
  it('reads files dropped from Finder or a browser', () => {
    const f = new File([new Uint8Array([1, 2])], 'photo.png', { type: 'image/png' });
    const r = readDrop(dt({ 'text/uri-list': 'https://example.com/photo.png' }, [f]));
    expect(r.files).toEqual([f]);
    expect(r.urls).toEqual(['https://example.com/photo.png']);
  });

  it('reads links dragged from a browser', () => {
    const r = readDrop(dt({ 'text/uri-list': '# comment\nhttps://a.example/x\r\nftp://nope\nhttps://b.example' }));
    expect(r.urls).toEqual(['https://a.example/x', 'https://b.example']);
    expect(readDrop(dt({ 'text/plain': ' https://c.example/page ' })).urls).toEqual(['https://c.example/page']);
  });

  it('parses uri lists', () => {
    expect(parseUriList('#c\n\nhttps://x.example\nnot a url')).toEqual(['https://x.example']);
  });

  it('keeps plain text', () => {
    const r = readDrop(dt({ 'text/plain': 'hello world' }));
    expect(r.urls).toEqual([]);
    expect(r.text).toBe('hello world');
  });

  it('finds inline images in dropped HTML', () => {
    const html = '<p><img alt="x" src="data:image/png;base64,iVBO\nRw0K"><img src="https://x/y.png"></p>';
    expect(dataImagesInHtml(html)).toEqual(['data:image/png;base64,iVBORw0K']);
    const f = dataUrlToFile('data:image/jpeg;base64,AAEC', 'pic');
    expect(f?.name).toBe('pic.jpg');
    expect(f?.size).toBe(3);
    expect(dataUrlToFile('data:text/html;base64,AAEC')).toBeNull();
  });

  it('recognises useful drags', () => {
    expect(isUsefulDrag(['Files'])).toBe(true);
    expect(isUsefulDrag(['text/uri-list'])).toBe(true);
    expect(isUsefulDrag(['application/x-custom'])).toBe(false);
    expect(isUsefulDrag(undefined)).toBe(false);
  });
});
