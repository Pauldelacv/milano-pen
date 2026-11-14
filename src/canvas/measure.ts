// Rendered heights of auto-height cards, so culling, marquee selection and
// zoom-to-fit know their real bounds. Unmeasured cards use an estimate.
import type { Card } from '../types/models';
import type { Rect } from './geometry';

const measured = new Map<string, number>();
const listeners = new Set<(id: string) => void>();

const ESTIMATES: Partial<Record<Card['kind'], number>> = {
  note: 96,
  board: 150,
  file: 72,
  link: 88,
};

export function setMeasuredHeight(id: string, h: number): void {
  if (measured.get(id) === h) return;
  measured.set(id, h);
  for (const l of listeners) l(id);
}

export function onMeasured(l: (id: string) => void): () => void {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function cardRect(c: Card): Rect {
  return { x: c.x, y: c.y, w: c.w, h: c.h ?? measured.get(c.id) ?? ESTIMATES[c.kind] ?? 120 };
}

/** Cards drawn on the canvas itself (lines and column contents are not). */
export function isCanvasCard(c: Card): boolean {
  return !c.parentCardId && c.kind !== 'line';
}
