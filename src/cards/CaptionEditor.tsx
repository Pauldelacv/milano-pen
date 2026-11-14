import { useEffect, useRef, useState } from 'react';
import { saveQueue } from '../app/actions';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';

/** A single-line field editing one string in a card's data, autosaved. */
export function CaptionEditor({
  cardId,
  value,
  placeholder,
  field,
  label = 'Edit',
  className = 'caption-input',
}: {
  cardId: string;
  value: string;
  placeholder: string;
  field: string;
  label?: string;
  className?: string;
}) {
  const [text, setText] = useState(value);
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    ref.current?.focus();
    ref.current?.select();
  }, []);
  const update = (v: string) => {
    setText(v);
    useBoardStore.getState().setLocal([{ id: cardId, data: { [field]: v } }]);
    saveQueue.schedule({ id: cardId, data: { [field]: v } }, label, `edit:${cardId}:${field}`);
  };
  return (
    <input
      ref={ref}
      data-no-drag
      className={className}
      value={text}
      placeholder={placeholder}
      onChange={(e) => update(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === 'Escape') {
          e.preventDefault();
          useUiStore.getState().setEditing(null);
          void saveQueue.flush();
        }
      }}
    />
  );
}
