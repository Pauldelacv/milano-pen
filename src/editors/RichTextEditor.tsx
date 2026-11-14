// The note editor, mounted only while a note is being edited.
import type { JSONContent } from '@tiptap/core';
import { EditorContent, useEditor, useEditorState, type Editor } from '@tiptap/react';
import { Bold, Heading1, Heading2, Italic, Link2, List, ListOrdered, Strikethrough, Underline } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { saveQueue } from '../app/actions';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';
import { FontPicker } from './FontPicker';
import { extensions } from './richText';

export default function RichTextEditor({ cardId, doc }: { cardId: string; doc: JSONContent }) {
  const editor = useEditor({
    extensions,
    content: doc,
    autofocus: 'end',
    editorProps: {
      attributes: { class: 'rich-editor', spellcheck: 'true' },
      handleKeyDown: (_view, event) => {
        if (event.key === 'Escape') {
          useUiStore.getState().setEditing(null);
          void saveQueue.flush();
          return true;
        }
        return false;
      },
    },
    onUpdate: ({ editor }) => {
      const data = { doc: editor.getJSON(), text: editor.getText({ blockSeparator: '\n' }) };
      useBoardStore.getState().setLocal([{ id: cardId, data }]);
      saveQueue.schedule({ id: cardId, data }, 'Edit note', `edit:${cardId}`);
    },
  });

  // Save as soon as editing ends (blur, Escape, click elsewhere).
  useEffect(() => () => void saveQueue.flush(), []);

  return (
    <>
      {editor && <FormatBar editor={editor} cardId={cardId} />}
      <EditorContent editor={editor} data-no-drag />
    </>
  );
}

function FormatBar({ editor, cardId }: { editor: Editor; cardId: string }) {
  const state = useEditorState({
    editor,
    selector: ({ editor: e }) => ({
      bold: e.isActive('bold'),
      italic: e.isActive('italic'),
      underline: e.isActive('underline'),
      strike: e.isActive('strike'),
      h1: e.isActive('heading', { level: 1 }),
      h2: e.isActive('heading', { level: 2 }),
      bullet: e.isActive('bulletList'),
      ordered: e.isActive('orderedList'),
      link: e.isActive('link'),
    }),
  });
  const [linkOpen, setLinkOpen] = useState(false);
  const [href, setHref] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);
  const chain = () => editor.chain().focus();
  const btn = (active: boolean, title: string, onClick: () => void, icon: React.ReactNode) => (
    <button
      type="button"
      className={`fmt${active ? ' active' : ''}`}
      title={title}
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
    >
      {icon}
    </button>
  );
  const applyLink = () => {
    const url = href.trim();
    if (!url) chain().extendMarkRange('link').unsetLink().run();
    else chain().extendMarkRange('link').setLink({ href: /^(https?:|mailto:)/i.test(url) ? url : `https://${url}` }).run();
    setLinkOpen(false);
  };
  return (
    <div className="format-bar" data-no-drag onPointerDown={(e) => e.stopPropagation()}>
      {linkOpen ? (
        <form
          className="link-form"
          onSubmit={(e) => {
            e.preventDefault();
            applyLink();
          }}
        >
          <input
            ref={inputRef}
            autoFocus
            value={href}
            placeholder="https://"
            onChange={(e) => setHref(e.target.value)}
            onKeyDown={(e) => e.key === 'Escape' && (e.stopPropagation(), setLinkOpen(false))}
          />
          <button type="submit" className="fmt-text">
            {href.trim() ? 'Apply' : 'Remove'}
          </button>
        </form>
      ) : (
        <>
          {btn(state.bold, 'Bold (⌘B)', () => chain().toggleBold().run(), <Bold size={14} />)}
          {btn(state.italic, 'Italic (⌘I)', () => chain().toggleItalic().run(), <Italic size={14} />)}
          {btn(state.underline, 'Underline (⌘U)', () => chain().toggleUnderline().run(), <Underline size={14} />)}
          {btn(state.strike, 'Strikethrough', () => chain().toggleStrike().run(), <Strikethrough size={14} />)}
          <span className="fmt-sep" />
          {btn(state.h1, 'Large heading', () => chain().toggleHeading({ level: 1 }).run(), <Heading1 size={14} />)}
          {btn(state.h2, 'Heading', () => chain().toggleHeading({ level: 2 }).run(), <Heading2 size={14} />)}
          {btn(state.bullet, 'Bulleted list', () => chain().toggleBulletList().run(), <List size={14} />)}
          {btn(state.ordered, 'Numbered list', () => chain().toggleOrderedList().run(), <ListOrdered size={14} />)}
          <span className="fmt-sep" />
          <FontPicker cardIds={[cardId]} compact />
          <span className="fmt-sep" />
          {btn(state.link, 'Link', () => {
            setHref(editor.getAttributes('link').href ?? '');
            setLinkOpen(true);
          }, <Link2 size={14} />)}
        </>
      )}
    </div>
  );
}
