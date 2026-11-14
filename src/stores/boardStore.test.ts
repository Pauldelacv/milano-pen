import { beforeEach, describe, expect, it } from 'vitest';
import type { Board, Card, Patch } from '../types/models';
import { setDataPinnedPredicate, useBoardStore } from './boardStore';

const board = (id: string, parent: string | null = null, title = id): Board => ({
  id, parentBoardId: parent, title, color: null, icon: null, createdAt: 0, updatedAt: 0, cardCount: 0,
});
const card = (id: string, boardId = 'home', data: Record<string, unknown> = {}): Card => ({
  id, boardId, parentCardId: null, kind: 'note', x: 0, y: 0, w: 240, h: null, z: 1, color: null, data, createdAt: 0, updatedAt: 0,
});
const patch = (p: Partial<Patch>): Patch => ({ cards: [], removedCards: [], boards: [], removedBoards: [], created: [], label: '', ...p });

describe('boardStore', () => {
  beforeEach(() => {
    setDataPinnedPredicate(() => false);
    useBoardStore.getState().load({
      board: board('home'),
      breadcrumb: [{ id: 'home', title: 'Home', color: null }],
      cards: [card('a'), card('b')],
      childBoards: [board('child', 'home')],
    });
  });

  it('applies created, moved-away and removed cards', () => {
    const s = useBoardStore.getState();
    s.applyPatch(patch({ cards: [card('c'), { ...card('a'), boardId: 'elsewhere' }], removedCards: ['b'] }));
    expect(Object.keys(useBoardStore.getState().cards)).toEqual(['c']);
  });

  it('updates the board, breadcrumb and child boards', () => {
    useBoardStore.getState().applyPatch(patch({ boards: [board('home', null, 'Renamed'), board('child', 'home', 'Kid')] }));
    const s = useBoardStore.getState();
    expect(s.board?.title).toBe('Renamed');
    expect(s.breadcrumb[0].title).toBe('Renamed');
    expect(s.childBoards.child.title).toBe('Kid');
    s.applyPatch(patch({ removedBoards: ['home'] }));
    expect(useBoardStore.getState().boardGone).toBe(true);
  });

  it('keeps unsaved local text when a stale patch arrives', () => {
    useBoardStore.getState().setLocal([{ id: 'a', data: { text: 'typed more' }, x: 50 }]);
    setDataPinnedPredicate((id) => id === 'a');
    useBoardStore.getState().applyPatch(patch({ cards: [{ ...card('a', 'home', { text: 'typed' }), x: 60 }] }));
    const a = useBoardStore.getState().cards.a;
    expect(a.data.text).toBe('typed more');
    expect(a.x).toBe(60);
  });
});
