// Left toolbar. Click a tool to add a card in the middle of the view, or drag
// it onto the canvas to place it. Only implemented card types are listed.
import { File, Image, LayoutGrid, Link2, StickyNote } from 'lucide-react';
import { useRef, useState, type ReactNode } from 'react';
import { addBoard, addFiles, addLink, addNote } from '../app/actions';
import { clientToWorld, stageRect } from '../canvas/viewport';

type ToolId = 'note' | 'link' | 'image' | 'file' | 'board';
type At = { x: number; y: number };

const TOOLS: { id: ToolId; label: string; icon: ReactNode; hint: string }[] = [
  { id: 'note', label: 'Note', icon: <StickyNote size={20} strokeWidth={1.6} />, hint: 'Add a note (⌘N)' },
  { id: 'link', label: 'Link', icon: <Link2 size={20} strokeWidth={1.6} />, hint: 'Add a link' },
  { id: 'image', label: 'Image', icon: <Image size={20} strokeWidth={1.6} />, hint: 'Add images (⇧⌘I)' },
  { id: 'file', label: 'File', icon: <File size={20} strokeWidth={1.6} />, hint: 'Add files' },
  { id: 'board', label: 'Board', icon: <LayoutGrid size={20} strokeWidth={1.6} />, hint: 'Add a board (⇧⌘N)' },
];

export function Toolbar() {
  const [ghost, setGhost] = useState<{ id: ToolId; x: number; y: number } | null>(null);
  const [linkAt, setLinkAt] = useState<At | null | undefined>(undefined);
  const [url, setUrl] = useState('');
  const drag = useRef<{ id: ToolId; x: number; y: number; moved: boolean } | null>(null);

  const place = (id: ToolId, at?: At) => {
    const pos = at && { x: Math.round(at.x - (id === 'board' ? 60 : 120)), y: Math.round(at.y - 20) };
    if (id === 'note') void addNote(pos);
    if (id === 'board') void addBoard(pos);
    if (id === 'image') void addFiles(true, at);
    if (id === 'file') void addFiles(false, at);
    if (id === 'link') {
      setUrl('');
      setLinkAt(pos ?? null);
    }
  };

  const onPointerDown = (id: ToolId) => (e: React.PointerEvent<HTMLButtonElement>) => {
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { id, x: e.clientX, y: e.clientY, moved: false };
  };
  const onPointerMove = (e: React.PointerEvent) => {
    const d = drag.current;
    if (!d) return;
    if (!d.moved && Math.hypot(e.clientX - d.x, e.clientY - d.y) < 6) return;
    d.moved = true;
    setGhost({ id: d.id, x: e.clientX, y: e.clientY });
  };
  const onPointerUp = (e: React.PointerEvent) => {
    const d = drag.current;
    drag.current = null;
    setGhost(null);
    if (!d) return;
    if (!d.moved) return place(d.id);
    const r = stageRect();
    if (e.clientX > r.left && e.clientX < r.right && e.clientY > r.top && e.clientY < r.bottom) {
      place(d.id, clientToWorld(e.clientX, e.clientY));
    }
  };

  return (
    <>
      <aside className="toolbar" aria-label="Add to board">
        {TOOLS.map((t) => (
          <button
            key={t.id}
            className="tool"
            title={t.hint}
            onPointerDown={onPointerDown(t.id)}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={() => ((drag.current = null), setGhost(null))}
            onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), place(t.id))}
          >
            {t.icon}
            <span>{t.label}</span>
          </button>
        ))}
      </aside>
      {ghost && (
        <div className="tool-ghost" style={{ left: ghost.x, top: ghost.y }}>
          {TOOLS.find((t) => t.id === ghost.id)?.icon}
        </div>
      )}
      {linkAt !== undefined && (
        <form
          className="link-popover"
          onSubmit={(e) => {
            e.preventDefault();
            void addLink(url, linkAt ?? undefined);
            setLinkAt(undefined);
          }}
        >
          <label htmlFor="link-url">Paste a web address</label>
          <input
            id="link-url"
            autoFocus
            value={url}
            placeholder="https://"
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => e.key === 'Escape' && setLinkAt(undefined)}
          />
          <div className="row">
            <button type="button" className="button" onClick={() => setLinkAt(undefined)}>
              Cancel
            </button>
            <button type="submit" className="button primary" disabled={!url.trim()}>
              Add link
            </button>
          </div>
        </form>
      )}
    </>
  );
}
