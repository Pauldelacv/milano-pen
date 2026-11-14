// Positions one card and picks its content component. Subscribes only to its
// own record, selection flag and editing flag, so other cards' changes never
// re-render it.
import { memo, useLayoutEffect, useRef } from 'react';
import { setMeasuredHeight } from '../canvas/measure';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';
import { cardComponent } from './registry';

export const CardView = memo(function CardView({ id }: { id: string }) {
  const card = useBoardStore((s) => s.cards[id]);
  const selected = useUiStore((s) => s.selection[id] === true);
  const editing = useUiStore((s) => s.editingId === id);
  const ref = useRef<HTMLDivElement>(null);
  const autoHeight = card?.h == null;

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el || !autoHeight) return;
    const ro = new ResizeObserver(() => setMeasuredHeight(id, el.offsetHeight));
    ro.observe(el);
    setMeasuredHeight(id, el.offsetHeight);
    return () => ro.disconnect();
  }, [id, autoHeight]);

  if (!card) return null;
  const Content = cardComponent(card.kind);
  const resizable = card.kind !== 'board';
  return (
    <div
      ref={ref}
      data-card-id={id}
      className={`card kind-${card.kind}${card.color ? ` color-${card.color}` : ''}${selected ? ' selected' : ''}${editing ? ' editing' : ''}`}
      style={{
        transform: `translate3d(${card.x}px, ${card.y}px, 0)`,
        width: card.w,
        height: card.h ?? undefined,
        zIndex: editing ? 1_000_000_000 : card.z,
      }}
    >
      <Content card={card} editing={editing} selected={selected} />
      {selected && !editing && resizable && <div className="resize-handle" data-resize title="Resize" />}
    </div>
  );
});
