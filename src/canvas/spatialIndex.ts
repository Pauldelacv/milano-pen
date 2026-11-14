import { intersects, type Rect } from './geometry';

const MAX_CELLS_PER_ITEM = 256;

/**
 * Uniform-grid spatial hash over card bounds. Insert/update/remove are
 * O(cells covered); a viewport query touches only the cells it overlaps, so
 * culling stays cheap with thousands of cards.
 */
export class SpatialIndex {
  private cells = new Map<string, Set<string>>();
  private rects = new Map<string, Rect>();
  private keys = new Map<string, string[]>();
  /** Items spanning too many cells to hash; checked on every query. */
  private huge = new Set<string>();

  constructor(private readonly cellSize = 512) {}

  get size(): number {
    return this.rects.size;
  }

  private range(r: Rect) {
    const s = this.cellSize;
    const x1 = Math.floor(r.x / s);
    const y1 = Math.floor(r.y / s);
    const x2 = Math.floor((r.x + Math.max(r.w, 0)) / s);
    const y2 = Math.floor((r.y + Math.max(r.h, 0)) / s);
    return { x1, y1, x2, y2, count: (x2 - x1 + 1) * (y2 - y1 + 1) };
  }

  private cellKeys(r: Rect): string[] {
    const { x1, y1, x2, y2 } = this.range(r);
    const out: string[] = [];
    for (let x = x1; x <= x2; x++) for (let y = y1; y <= y2; y++) out.push(`${x},${y}`);
    return out;
  }

  set(id: string, r: Rect): void {
    const prev = this.rects.get(id);
    if (prev && prev.x === r.x && prev.y === r.y && prev.w === r.w && prev.h === r.h) return;
    this.remove(id);
    this.rects.set(id, { ...r });
    if (this.range(r).count > MAX_CELLS_PER_ITEM) {
      this.huge.add(id);
      return;
    }
    const keys = this.cellKeys(r);
    for (const k of keys) {
      let cell = this.cells.get(k);
      if (!cell) this.cells.set(k, (cell = new Set()));
      cell.add(id);
    }
    this.keys.set(id, keys);
  }

  remove(id: string): void {
    this.huge.delete(id);
    this.rects.delete(id);
    const keys = this.keys.get(id);
    if (!keys) return;
    for (const k of keys) {
      const cell = this.cells.get(k);
      cell?.delete(id);
      if (cell && cell.size === 0) this.cells.delete(k);
    }
    this.keys.delete(id);
  }

  get(id: string): Rect | undefined {
    return this.rects.get(id);
  }

  clear(): void {
    this.cells.clear();
    this.rects.clear();
    this.keys.clear();
    this.huge.clear();
  }

  /** Ids whose bounds intersect `r`. */
  query(r: Rect): string[] {
    const found = new Set<string>();
    // A query wider than the occupied grid: a linear scan is cheaper.
    if (this.range(r).count > this.cells.size) {
      for (const [id, rect] of this.rects) if (intersects(r, rect)) found.add(id);
      return [...found];
    }
    for (const id of this.huge) if (intersects(r, this.rects.get(id)!)) found.add(id);
    for (const k of this.cellKeys(r)) {
      const cell = this.cells.get(k);
      if (!cell) continue;
      for (const id of cell) {
        if (!found.has(id) && intersects(r, this.rects.get(id)!)) found.add(id);
      }
    }
    return [...found];
  }

  all(): Rect[] {
    return [...this.rects.values()];
  }
}
