import { useLayoutEffect, useRef, useState } from 'react';
import {
  activateCard, addBoard, addFiles, addNote, copySelection, deleteCards, duplicateSelection, pasteCards, restack,
  run, selectAll, setColor, updateData, zoomToFit,
} from '../app/actions';
import { BOARD_ICONS, BoardIcon } from '../cards/boardIcons';
import { COLORS } from '../cards/colors';
import { api } from '../services/ipc';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';

type Item = { label: string; shortcut?: string; run: () => void; danger?: boolean } | 'sep';

export function ContextMenu() {
  const menu = useUiStore((s) => s.contextMenu);
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x: 0, y: 0 });
  const cards = useBoardStore((s) => s.cards);

  useLayoutEffect(() => {
    if (!menu || !ref.current) return;
    const r = ref.current.getBoundingClientRect();
    setPos({ x: Math.min(menu.x, window.innerWidth - r.width - 8), y: Math.min(menu.y, window.innerHeight - r.height - 8) });
  }, [menu]);

  if (!menu) return null;
  const close = () => useUiStore.getState().setContextMenu(null);
  const ids = menu.cardIds.filter((id) => cards[id]);
  const one = ids.length === 1 ? cards[ids[0]] : null;
  const at = { x: Math.round(menu.world.x), y: Math.round(menu.world.y) };
  const edit = (id: string) => {
    useUiStore.getState().select([id]);
    useUiStore.getState().setEditing(id);
  };

  const items: Item[] = [];
  if (!ids.length) {
    items.push(
      { label: 'Add note here', run: () => void addNote({ x: at.x - 120, y: at.y - 20 }) },
      { label: 'Add board here', run: () => void addBoard({ x: at.x - 60, y: at.y - 20 }) },
      { label: 'Add images…', run: () => void addFiles(true, at) },
      { label: 'Add files…', run: () => void addFiles(false, at) },
      'sep',
      { label: 'Paste', shortcut: '⌘V', run: () => void pasteCards(at) },
      { label: 'Select all', shortcut: '⌘A', run: selectAll },
      { label: 'Zoom to fit', shortcut: '⌘1', run: () => zoomToFit() },
    );
  } else {
    if (one?.kind === 'note') items.push({ label: 'Edit', shortcut: '↩', run: () => edit(one.id) });
    if (one?.kind === 'board') {
      items.push({ label: 'Open board', shortcut: '↩', run: () => void activateCard(one.id) });
      items.push({ label: 'Rename', run: () => edit(one.id) });
    }
    if (one?.kind === 'image') {
      items.push({ label: 'View full size', shortcut: '↩', run: () => void activateCard(one.id) });
      items.push({ label: one.data.caption ? 'Edit caption' : 'Add caption', run: () => edit(one.id) });
    }
    if (one?.kind === 'link') {
      items.push({ label: 'Open link', shortcut: '↩', run: () => void activateCard(one.id) });
      items.push({ label: 'Edit title', run: () => edit(one.id) });
    }
    if ((one?.kind === 'file' || one?.kind === 'image') && typeof one.data.assetId === 'string') {
      const assetId = one.data.assetId;
      if (one.kind === 'file') items.push({ label: 'Open', shortcut: '↩', run: () => void activateCard(one.id) });
      items.push({ label: 'Show in Finder', run: () => void run(api.revealAsset(assetId)) });
    }
    if (items.length) items.push('sep');
    items.push(
      { label: 'Duplicate', shortcut: '⌘D', run: () => void duplicateSelection() },
      { label: 'Copy', shortcut: '⌘C', run: () => void copySelection(null) },
      { label: 'Cut', shortcut: '⌘X', run: () => void copySelection(null, true) },
      'sep',
      { label: 'Bring to front', shortcut: '⇧⌘]', run: () => void restack(true, ids) },
      { label: 'Send to back', shortcut: '⇧⌘[', run: () => void restack(false, ids) },
      'sep',
      { label: ids.length > 1 ? `Delete ${ids.length} cards` : 'Delete', shortcut: '⌫', danger: true, run: () => void deleteCards(ids) },
    );
  }

  const showColors = ids.length > 0;
  const showSize = ids.length > 0 && ids.every((id) => cards[id]?.kind === 'note');
  return (
    <div
      ref={ref}
      className="context-menu"
      role="menu"
      style={{ left: pos.x, top: pos.y }}
      onPointerDown={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {showColors && (
        <div className="menu-colors" aria-label="Colour">
          <button className="swatch none" title="No colour" onClick={() => (void setColor(ids, null), close())} />
          {COLORS.map((c) => (
            <button key={c} className="swatch" title={c} style={{ background: `var(--c-${c})` }} onClick={() => (void setColor(ids, c), close())} />
          ))}
        </div>
      )}
      {showSize && (
        <div className="menu-segment" aria-label="Text size">
          {(['small', 'normal', 'large'] as const).map((s) => (
            <button key={s} onClick={() => (ids.forEach((id) => void updateData(id, { textSize: s }, 'Text size')), close())}>
              {s === 'small' ? 'Small' : s === 'normal' ? 'Normal' : 'Large'}
            </button>
          ))}
        </div>
      )}
      {one?.kind === 'board' && (
        <div className="menu-icons" aria-label="Board icon">
          {Object.keys(BOARD_ICONS).map((name) => (
            <button key={name} title={name} onClick={() => (void updateData(one.id, { icon: name }, 'Change icon'), close())}>
              <BoardIcon name={name} size={14} />
            </button>
          ))}
        </div>
      )}
      {items.map((it, i) =>
        it === 'sep' ? (
          <div key={i} className="menu-sep" />
        ) : (
          <button
            key={i}
            role="menuitem"
            className={`menu-item${it.danger ? ' danger' : ''}`}
            onClick={() => {
              close();
              it.run();
            }}
          >
            <span>{it.label}</span>
            {it.shortcut && <kbd>{it.shortcut}</kbd>}
          </button>
        ),
      )}
    </div>
  );
}
