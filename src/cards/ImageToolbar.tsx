// Floating toolbar above a selected image: colour adjustments (brightness,
// contrast, saturation, hue, black & white, sepia), caption and full-size view.
// Drawn in world space but counter-scaled, so it stays the same size on screen.
import { Expand, MessageSquareText, RotateCcw, SlidersHorizontal } from 'lucide-react';
import { useState } from 'react';
import { activateCard, saveQueue } from '../app/actions';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';
import type { ImageAdjust } from '../types/models';
import { ADJUST_CONTROLS, adjustValue, isNeutral, type AdjustKey } from './imageAdjust';

function setAdjust(cardId: string, adjust: ImageAdjust | null) {
  useBoardStore.getState().setLocal([{ id: cardId, data: { adjust } }]);
  // One undo step per burst of slider movement.
  saveQueue.schedule({ id: cardId, data: { adjust } }, 'Adjust image', `adjust:${cardId}`);
}

export function ImageToolbar({ cardId, adjust }: { cardId: string; adjust: ImageAdjust | undefined }) {
  const [open, setOpen] = useState(false);
  const change = (key: AdjustKey, value: number) => {
    const next: ImageAdjust = { ...(adjust ?? {}), [key]: value };
    setAdjust(cardId, isNeutral(next) ? null : next);
  };
  return (
    <div
      className="card-toolbar image-toolbar"
      data-no-drag
      onPointerDown={(e) => e.stopPropagation()}
      onDoubleClick={(e) => e.stopPropagation()}
    >
      <div className="card-toolbar-row">
        <button className={`icon-button${open ? ' active' : ''}`} title="Adjust colours" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
          <SlidersHorizontal size={14} />
          <span>Adjust</span>
        </button>
        <button
          className="icon-button"
          title="Caption"
          onClick={() => {
            useUiStore.getState().select([cardId]);
            useUiStore.getState().setEditing(cardId);
          }}
        >
          <MessageSquareText size={14} />
        </button>
        <button className="icon-button" title="View full size" onClick={() => void activateCard(cardId)}>
          <Expand size={14} />
        </button>
        {!isNeutral(adjust) && (
          <button
            className="icon-button"
            title="Reset adjustments"
            onClick={() => {
              setAdjust(cardId, null);
              void saveQueue.flush();
            }}
          >
            <RotateCcw size={14} />
          </button>
        )}
      </div>
      {open && (
        <div className="adjust-panel">
          {ADJUST_CONTROLS.map((c) => {
            const v = adjustValue(adjust, c.key);
            return (
              <label key={c.key} className="adjust-row">
                <span className="adjust-label">{c.label}</span>
                <input
                  type="range"
                  min={c.min}
                  max={c.max}
                  step={1}
                  value={v}
                  onChange={(e) => change(c.key, Number(e.target.value))}
                  onPointerUp={() => void saveQueue.flush()}
                  onDoubleClick={() => change(c.key, c.neutral)}
                />
                <span className="adjust-value">
                  {c.unit === '%' ? `${v}%` : `${v}°`}
                </span>
              </label>
            );
          })}
        </div>
      )}
    </div>
  );
}
