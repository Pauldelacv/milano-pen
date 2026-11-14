import { describe, expect, it } from 'vitest';
import { centerOn, fitRect, screenToWorld, visibleWorldRect, worldToScreen, zoomAt, MAX_SCALE } from './camera';
import { expand, intersects, rectFromPoints, snap, union } from './geometry';
import { SpatialIndex } from './spatialIndex';

describe('camera', () => {
  it('round-trips screen and world coordinates', () => {
    const cam = { x: 120, y: -40, scale: 1.5 };
    const p = { x: 33, y: 77 };
    const back = worldToScreen(cam, screenToWorld(cam, p));
    expect(back.x).toBeCloseTo(p.x);
    expect(back.y).toBeCloseTo(p.y);
  });

  it('zooms around the anchor point', () => {
    const cam = { x: 10, y: 20, scale: 1 };
    const anchor = { x: 300, y: 200 };
    const before = screenToWorld(cam, anchor);
    const after = screenToWorld(zoomAt(cam, anchor, 2), anchor);
    expect(after.x).toBeCloseTo(before.x);
    expect(after.y).toBeCloseTo(before.y);
    expect(zoomAt(cam, anchor, 1000).scale).toBe(MAX_SCALE);
  });

  it('computes the visible world rect, fits and centres', () => {
    expect(visibleWorldRect({ x: -100, y: -50, scale: 2 }, 800, 600)).toEqual({ x: 50, y: 25, w: 400, h: 300 });
    const fit = fitRect({ x: 0, y: 0, w: 2000, h: 1000 }, 1000, 600, 0);
    expect(fit.scale).toBeCloseTo(0.5);
    expect(fitRect({ x: 0, y: 0, w: 10, h: 10 }, 1000, 600).scale).toBe(1);
    const c = centerOn({ x: 0, y: 0, scale: 2 }, { x: 100, y: 100 }, 800, 600);
    expect(worldToScreen(c, { x: 100, y: 100 })).toEqual({ x: 400, y: 300 });
  });
});

describe('geometry', () => {
  it('handles rectangles', () => {
    expect(intersects({ x: 0, y: 0, w: 10, h: 10 }, { x: 9, y: 9, w: 5, h: 5 })).toBe(true);
    expect(intersects({ x: 0, y: 0, w: 10, h: 10 }, { x: 10, y: 0, w: 5, h: 5 })).toBe(false);
    expect(rectFromPoints({ x: 10, y: 5 }, { x: 0, y: 20 })).toEqual({ x: 0, y: 5, w: 10, h: 15 });
    expect(expand({ x: 0, y: 0, w: 10, h: 10 }, 5)).toEqual({ x: -5, y: -5, w: 20, h: 20 });
    expect(union([{ x: 0, y: 0, w: 1, h: 1 }, { x: 5, y: -2, w: 1, h: 1 }])).toEqual({ x: 0, y: -2, w: 6, h: 3 });
    expect(union([])).toBeNull();
    expect(snap(14, 10)).toBe(10);
    expect(snap(14, 10, false)).toBe(14);
  });
});

describe('SpatialIndex', () => {
  it('finds, moves and removes items', () => {
    const idx = new SpatialIndex(100);
    idx.set('a', { x: 0, y: 0, w: 50, h: 50 });
    idx.set('b', { x: 1000, y: 1000, w: 50, h: 50 });
    idx.set('big', { x: -500, y: -500, w: 2000, h: 2000 });
    expect(idx.query({ x: 10, y: 10, w: 5, h: 5 }).sort()).toEqual(['a', 'big']);
    idx.set('a', { x: 990, y: 990, w: 20, h: 20 });
    expect(idx.query({ x: 0, y: 0, w: 60, h: 60 })).toEqual(['big']);
    expect(idx.query({ x: 995, y: 995, w: 10, h: 10 }).sort()).toEqual(['a', 'b', 'big']);
    idx.set('giant', { x: -1e7, y: -1e7, w: 2e7, h: 10 });
    expect(idx.query({ x: 0, y: -1e7, w: 1, h: 5 })).toEqual(['giant']);
    idx.remove('giant');
    idx.remove('big');
    expect(idx.size).toBe(2);
    idx.clear();
    expect(idx.query({ x: -1e6, y: -1e6, w: 2e6, h: 2e6 })).toEqual([]);
  });

  it('culls a large board quickly', () => {
    const idx = new SpatialIndex();
    for (let i = 0; i < 20000; i++) idx.set(String(i), { x: (i % 200) * 300, y: Math.floor(i / 200) * 300, w: 240, h: 160 });
    const t = performance.now();
    let found = 0;
    for (let i = 0; i < 100; i++) found = idx.query({ x: i * 50, y: i * 50, w: 1600, h: 1000 }).length;
    expect(found).toBeGreaterThan(0);
    expect(found).toBeLessThan(100);
    expect(performance.now() - t).toBeLessThan(500);
  });
});
