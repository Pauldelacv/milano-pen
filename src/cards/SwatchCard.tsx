import { Copy } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { saveQueue } from '../app/actions';
import { copyText } from '../services/clipboard';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';
import type { SwatchData } from '../types/models';
import { parseColor } from './colorParse';
import type { CardProps } from './registry';

export const DEFAULT_SWATCH = '#4C6EF5';

export async function copyHex(hex: string): Promise<void> {
  const ok = await copyText(hex);
  useUiStore.getState().toast(ok ? `Copied ${hex}` : 'Couldn’t copy to the clipboard', { kind: ok ? 'info' : 'error' });
}

function save(cardId: string, hex: string) {
  useBoardStore.getState().setLocal([{ id: cardId, data: { hex } }]);
  saveQueue.schedule({ id: cardId, data: { hex } }, 'Change colour', `edit:${cardId}:hex`);
}

function SwatchEditor({ cardId, hex }: { cardId: string; hex: string }) {
  const [text, setText] = useState(hex);
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    ref.current?.focus();
    ref.current?.select();
  }, []);
  const valid = parseColor(text);
  const done = () => {
    useUiStore.getState().setEditing(null);
    void saveQueue.flush();
  };
  return (
    <div className="swatch-edit" data-no-drag>
      <input
        ref={ref}
        className={`caption-input${valid ? '' : ' invalid'}`}
        value={text}
        aria-label="Colour (hex, rgb, hsl or name)"
        placeholder="#RRGGBB, rgb(), hsl() or a name"
        spellCheck={false}
        onChange={(e) => {
          setText(e.target.value);
          const c = parseColor(e.target.value);
          if (c) save(cardId, c);
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === 'Escape') {
            e.preventDefault();
            done();
          }
        }}
      />
      <input
        type="color"
        className="swatch-picker"
        aria-label="Pick a colour"
        value={(valid ?? hex).toLowerCase()}
        onChange={(e) => {
          const c = e.target.value.toUpperCase();
          setText(c);
          save(cardId, c);
        }}
      />
    </div>
  );
}

export function SwatchCard({ card, editing }: CardProps) {
  const data = card.data as unknown as SwatchData;
  const hex = parseColor(data.hex ?? '') ?? DEFAULT_SWATCH;
  return (
    <div className="swatch-card">
      <div className="swatch-block" style={{ background: hex }} />
      {editing ? (
        <SwatchEditor cardId={card.id} hex={hex} />
      ) : (
        <button className="swatch-hex" data-no-drag title="Copy to clipboard" onClick={() => void copyHex(hex)}>
          <span>{hex}</span>
          <Copy size={12} />
        </button>
      )}
    </div>
  );
}
