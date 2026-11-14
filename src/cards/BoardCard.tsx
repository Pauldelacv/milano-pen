import { useBoardStore } from '../stores/boardStore';
import type { BoardCardData } from '../types/models';
import { BoardIcon } from './boardIcons';
import { BoardTitleEditor } from './BoardTitleEditor';
import type { CardProps } from './registry';

export function BoardCard({ card, editing }: CardProps) {
  const childId = (card.data as unknown as BoardCardData).childBoardId;
  const child = useBoardStore((s) => s.childBoards[childId]);
  const color = child?.color ?? card.color ?? 'slate';
  const count = child?.cardCount ?? 0;
  return (
    <div className="board-card" title="Double-click to open">
      <div className="board-tile" style={{ background: `var(--c-${color}, var(--c-slate))` }}>
        <BoardIcon name={child?.icon} size={34} />
      </div>
      {editing ? (
        <BoardTitleEditor cardId={card.id} value={child?.title ?? ''} />
      ) : (
        <div className="board-title">{child?.title ?? 'Board'}</div>
      )}
      <div className="board-count">{count === 1 ? '1 card' : `${count} cards`}</div>
    </div>
  );
}
