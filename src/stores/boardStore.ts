// The board currently on screen: its cards and the boards its board cards
// open. Updated incrementally from Rust patches, never reloaded per edit.
import { create } from 'zustand';
import type { Board, BoardView, Card, Crumb, Patch } from '../types/models';

export interface LocalChange {
  id: string;
  x?: number;
  y?: number;
  w?: number;
  h?: number | null;
  color?: string | null;
  data?: Record<string, unknown>;
}

interface BoardState {
  boardId: string | null;
  board: Board | null;
  breadcrumb: Crumb[];
  cards: Record<string, Card>;
  childBoards: Record<string, Board>;
  /** Set when the open board was deleted (e.g. undo of its creation). */
  boardGone: boolean;
  load: (view: BoardView) => void;
  applyPatch: (patch: Patch) => void;
  /** Optimistic local changes (drag, resize, typing) before they are saved. */
  setLocal: (changes: LocalChange[]) => void;
}

/** Cards whose `data` has unsaved local edits; patches must not overwrite it. */
let isDataPinned: (id: string) => boolean = () => false;
export function setDataPinnedPredicate(fn: (id: string) => boolean) {
  isDataPinned = fn;
}

export const useBoardStore = create<BoardState>((set) => ({
  boardId: null,
  board: null,
  breadcrumb: [],
  cards: {},
  childBoards: {},
  boardGone: false,

  load: (view) =>
    set({
      boardId: view.board.id,
      board: view.board,
      breadcrumb: view.breadcrumb,
      cards: Object.fromEntries(view.cards.map((c) => [c.id, c])),
      childBoards: Object.fromEntries(view.childBoards.map((b) => [b.id, b])),
      boardGone: false,
    }),

  applyPatch: (patch) =>
    set((s) => {
      if (!s.boardId) return s;
      let cards = s.cards;
      const edit = () => (cards === s.cards ? (cards = { ...s.cards }) : cards);
      for (const c of patch.cards) {
        if (c.boardId === s.boardId) {
          const local = s.cards[c.id];
          edit()[c.id] = local && isDataPinned(c.id) ? { ...c, data: local.data } : c;
        } else if (cards[c.id]) {
          delete edit()[c.id];
        }
      }
      for (const id of patch.removedCards) if (cards[id]) delete edit()[id];

      let { board, breadcrumb, childBoards, boardGone } = s;
      for (const b of patch.boards) {
        if (b.id === s.boardId) board = b;
        if (b.parentBoardId === s.boardId || childBoards[b.id]) childBoards = { ...childBoards, [b.id]: b };
        if (breadcrumb.some((c) => c.id === b.id))
          breadcrumb = breadcrumb.map((c) => (c.id === b.id ? { ...c, title: b.title, color: b.color } : c));
      }
      for (const id of patch.removedBoards) {
        if (id === s.boardId || breadcrumb.some((c) => c.id === id)) boardGone = true;
        if (childBoards[id]) {
          childBoards = { ...childBoards };
          delete childBoards[id];
        }
      }
      return { cards, board, breadcrumb, childBoards, boardGone };
    }),

  setLocal: (changes) =>
    set((s) => {
      const cards = { ...s.cards };
      for (const ch of changes) {
        const c = cards[ch.id];
        if (!c) continue;
        cards[ch.id] = {
          ...c,
          ...(ch.x !== undefined && { x: ch.x }),
          ...(ch.y !== undefined && { y: ch.y }),
          ...(ch.w !== undefined && { w: ch.w }),
          ...(ch.h !== undefined && { h: ch.h }),
          ...(ch.color !== undefined && { color: ch.color }),
          ...(ch.data && { data: { ...c.data, ...ch.data } }),
        };
      }
      return { cards };
    }),
}));
