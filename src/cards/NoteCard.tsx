import { lazy, Suspense } from 'react';
import { fontStack } from '../editors/fonts';
import { docToHtml, isEmptyDoc, noteDoc } from '../editors/richText';
import type { NoteData } from '../types/models';
import type { CardProps } from './registry';

// The editor bundle (Tiptap/ProseMirror view) loads the first time a note is edited.
const RichTextEditor = lazy(() => import('../editors/RichTextEditor'));

export function NoteCard({ card, editing }: CardProps) {
  const data = card.data as NoteData;
  const doc = noteDoc(data);
  const size = data.textSize ?? 'normal';
  const style = { fontFamily: fontStack(data.font) };
  if (editing) {
    return (
      <div className={`note-body rich text-${size}`} style={style}>
        <Suspense fallback={<div className="rich-static" dangerouslySetInnerHTML={{ __html: docToHtml(doc) }} />}>
          <RichTextEditor cardId={card.id} doc={doc} />
        </Suspense>
      </div>
    );
  }
  return (
    <div className={`note-body rich text-${size}`} style={style}>
      {isEmptyDoc(doc) ? (
        <p className="placeholder">Empty note</p>
      ) : (
        // HTML generated from the Tiptap schema, never stored HTML.
        <div className="rich-static" dangerouslySetInnerHTML={{ __html: docToHtml(doc) }} />
      )}
    </div>
  );
}
