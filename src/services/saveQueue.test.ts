import { describe, expect, it, vi } from 'vitest';
import type { CardUpdate, Patch } from '../types/models';
import { SaveQueue } from './saveQueue';

const emptyPatch: Patch = { cards: [], removedCards: [], boards: [], removedBoards: [], created: [], label: '' };

describe('SaveQueue', () => {
  it('debounces and merges edits per card', async () => {
    vi.useFakeTimers();
    const sent: CardUpdate[][] = [];
    const busy: boolean[] = [];
    const q = new SaveQueue({
      delayMs: 400,
      send: async (u) => {
        sent.push(u);
        return emptyPatch;
      },
      onPatch: () => {},
      onError: () => {},
      onBusyChange: (b) => busy.push(b),
    });
    q.schedule({ id: 'a', data: { text: 'h' } }, 'Edit note', 'edit:a');
    q.schedule({ id: 'a', data: { text: 'he', doc: 1 } }, 'Edit note', 'edit:a');
    q.schedule({ id: 'a', x: 5 }, 'Edit note');
    expect(q.isPending('a')).toBe(true);
    expect(sent).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(400);
    expect(sent).toEqual([[{ id: 'a', x: 5, data: { text: 'he', doc: 1 } }]]);
    expect(q.isPending('a')).toBe(false);
    expect(busy).toEqual([true, false]);
    vi.useRealTimers();
  });

  it('flushes immediately and reports errors', async () => {
    const onError = vi.fn();
    const onPatch = vi.fn();
    const q = new SaveQueue({
      delayMs: 10_000,
      send: async (u) => {
        if (u[0].id === 'bad') throw new Error('nope');
        return emptyPatch;
      },
      onPatch,
      onError,
    });
    q.schedule({ id: 'ok', x: 1 }, 'Move', 'nudge');
    q.schedule({ id: 'bad', x: 1 }, 'Move', 'other');
    await q.flush();
    expect(onPatch).toHaveBeenCalledTimes(1);
    expect(onError).toHaveBeenCalledTimes(1);
    expect(q.busy).toBe(false);
  });
});
