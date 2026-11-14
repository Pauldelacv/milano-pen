import { ChevronRight, Minus, Plus, Redo2, Search, Settings, Trash2, Undo2 } from 'lucide-react';
import { useState } from 'react';
import { applyPatch, openBoard, redo, run, undo, zoomIn, zoomOut, zoomReset } from '../app/actions';
import { useZoom } from '../canvas/cameraController';
import { api } from '../services/ipc';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';

function CurrentTitle({ id, title }: { id: string; title: string }) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState(title);
  if (!editing) {
    return (
      <button
        className="crumb current"
        title="Rename board"
        onClick={() => {
          setText(title);
          setEditing(true);
        }}
      >
        {title}
      </button>
    );
  }
  const commit = async (save: boolean) => {
    setEditing(false);
    const t = text.trim();
    if (save && t && t !== title) applyPatch(await run(api.updateBoard(id, { title: t })));
  };
  return (
    <input
      className="crumb-input"
      autoFocus
      value={text}
      aria-label="Board name"
      onChange={(e) => setText(e.target.value)}
      onFocus={(e) => e.target.select()}
      onBlur={() => void commit(true)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') e.currentTarget.blur();
        if (e.key === 'Escape') void commit(false);
      }}
    />
  );
}

export function TopBar() {
  const crumbs = useBoardStore((s) => s.breadcrumb);
  const boardId = useBoardStore((s) => s.boardId);
  const history = useUiStore((s) => s.history);
  const saving = useUiStore((s) => s.saving);
  const setDialog = useUiStore((s) => s.setDialog);
  const scale = useZoom((s) => s.scale);
  return (
    <header className="topbar" data-tauri-drag-region>
      <nav className="crumbs" aria-label="Boards">
        {crumbs.map((c, i) => {
          const last = i === crumbs.length - 1;
          return (
            <span key={c.id} className="crumb-wrap">
              {i > 0 && <ChevronRight size={14} className="crumb-sep" />}
              {last && boardId ? (
                <CurrentTitle id={c.id} title={c.title} />
              ) : (
                <button className="crumb" onClick={() => void openBoard(c.id)}>
                  {c.title}
                </button>
              )}
            </span>
          );
        })}
      </nav>
      <div className="topbar-spacer" data-tauri-drag-region />
      <span className={`save-state${saving ? ' busy' : ''}`} aria-live="polite">
        {saving ? 'Saving…' : 'Saved'}
      </span>
      <div className="topbar-group">
        <button className="icon-button" title={history.undoLabel ? `Undo ${history.undoLabel} (⌘Z)` : 'Undo (⌘Z)'} disabled={!history.canUndo} onClick={() => void undo()}>
          <Undo2 size={16} />
        </button>
        <button className="icon-button" title={history.redoLabel ? `Redo ${history.redoLabel} (⇧⌘Z)` : 'Redo (⇧⌘Z)'} disabled={!history.canRedo} onClick={() => void redo()}>
          <Redo2 size={16} />
        </button>
      </div>
      <div className="topbar-group zoom">
        <button className="icon-button" title="Zoom out (⌘−)" onClick={zoomOut}>
          <Minus size={14} />
        </button>
        <button className="zoom-level" title="Actual size (⌘0)" onClick={zoomReset}>
          {Math.round(scale * 100)}%
        </button>
        <button className="icon-button" title="Zoom in (⌘=)" onClick={zoomIn}>
          <Plus size={14} />
        </button>
      </div>
      <div className="topbar-group">
        <button className="search-button" title="Search (⌘K)" onClick={() => setDialog('search')}>
          <Search size={14} />
          <span>Search</span>
          <kbd>⌘K</kbd>
        </button>
        <button className="icon-button" title="Trash" onClick={() => setDialog('trash')}>
          <Trash2 size={16} />
        </button>
        <button className="icon-button" title="Settings (⌘,)" onClick={() => setDialog('settings')}>
          <Settings size={16} />
        </button>
      </div>
    </header>
  );
}
