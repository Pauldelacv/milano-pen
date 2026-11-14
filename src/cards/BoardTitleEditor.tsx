import { useEffect, useRef, useState } from 'react';
import { applyPatch, run } from '../app/actions';
import { api } from '../services/ipc';
import { useUiStore } from '../stores/uiStore';

export function BoardTitleEditor({ cardId, value }: { cardId: string; value: string }) {
  const [text, setText] = useState(value);
  const ref = useRef<HTMLInputElement>(null);
  const done = useRef(false);
  useEffect(() => {
    ref.current?.focus();
    ref.current?.select();
  }, []);
  const commit = async (save: boolean) => {
    if (done.current) return;
    done.current = true;
    useUiStore.getState().setEditing(null);
    const title = text.trim();
    if (save && title && title !== value) applyPatch(await run(api.updateCards([{ id: cardId, data: { title } }], 'Rename board')));
  };
  return (
    <input
      ref={ref}
      data-no-drag
      className="board-title-input"
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={() => void commit(true)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') void commit(true);
        if (e.key === 'Escape') void commit(false);
      }}
    />
  );
}
