import { ChevronLeft, ChevronRight, X } from 'lucide-react';
import { useEffect, useMemo } from 'react';
import { assetUrl } from '../services/assets';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';
import type { ImageData } from '../types/models';

/** Full-size image viewer; ←/→ step through the board's images in reading order. */
export function ImageViewer() {
  const id = useUiStore((s) => s.viewerId);
  const cards = useBoardStore((s) => s.cards);
  const images = useMemo(
    () => Object.values(cards).filter((c) => c.kind === 'image' && !c.parentCardId).sort((a, b) => a.y - b.y || a.x - b.x),
    [cards],
  );
  const index = images.findIndex((c) => c.id === id);
  const card = images[index];
  const set = (i: number) => useUiStore.getState().setViewer(images[(i + images.length) % images.length]?.id ?? null);

  useEffect(() => {
    if (!id) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') useUiStore.getState().setViewer(null);
      else if (e.key === 'ArrowRight') set(index + 1);
      else if (e.key === 'ArrowLeft') set(index - 1);
      else return;
      e.preventDefault();
      e.stopPropagation();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  });

  if (!id || !card) return null;
  const data = card.data as unknown as ImageData;
  return (
    <div className="viewer" role="dialog" aria-label="Image viewer" onPointerDown={(e) => e.target === e.currentTarget && useUiStore.getState().setViewer(null)}>
      <img key={card.id} src={assetUrl(data.assetId, 'original')} alt={data.caption || data.name || ''} decoding="async" />
      {(data.caption || data.name) && <div className="viewer-caption">{data.caption || data.name}</div>}
      <button className="viewer-close" aria-label="Close" onClick={() => useUiStore.getState().setViewer(null)}>
        <X size={20} />
      </button>
      {images.length > 1 && (
        <>
          <button className="viewer-nav prev" aria-label="Previous image" onClick={() => set(index - 1)}>
            <ChevronLeft size={28} />
          </button>
          <button className="viewer-nav next" aria-label="Next image" onClick={() => set(index + 1)}>
            <ChevronRight size={28} />
          </button>
          <div className="viewer-count">
            {index + 1} / {images.length}
          </div>
        </>
      )}
    </div>
  );
}
