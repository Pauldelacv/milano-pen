import type { Point, Rect } from './geometry';

/** Screen = world × scale + (x, y). */
export interface Camera {
  x: number;
  y: number;
  scale: number;
}

export const MIN_SCALE = 0.1;
export const MAX_SCALE = 4;
export const DEFAULT_CAMERA: Camera = { x: 80, y: 60, scale: 1 };

export const clampScale = (s: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, s));

export function screenToWorld(cam: Camera, p: Point): Point {
  return { x: (p.x - cam.x) / cam.scale, y: (p.y - cam.y) / cam.scale };
}

export function worldToScreen(cam: Camera, p: Point): Point {
  return { x: p.x * cam.scale + cam.x, y: p.y * cam.scale + cam.y };
}

/** Zoom by `factor`, keeping the world point under `anchor` (screen) fixed. */
export function zoomAt(cam: Camera, anchor: Point, factor: number): Camera {
  const scale = clampScale(cam.scale * factor);
  const world = screenToWorld(cam, anchor);
  return { scale, x: anchor.x - world.x * scale, y: anchor.y - world.y * scale };
}

/** The world rectangle visible in a viewport of the given size. */
export function visibleWorldRect(cam: Camera, width: number, height: number): Rect {
  return { x: -cam.x / cam.scale, y: -cam.y / cam.scale, w: width / cam.scale, h: height / cam.scale };
}

/** Frame `bounds` in the viewport with `padding`, never magnifying past `maxScale`. */
export function fitRect(bounds: Rect, width: number, height: number, padding = 60, maxScale = 1): Camera {
  const scale = clampScale(
    Math.min(maxScale, (width - 2 * padding) / Math.max(bounds.w, 1), (height - 2 * padding) / Math.max(bounds.h, 1)),
  );
  return {
    scale,
    x: (width - bounds.w * scale) / 2 - bounds.x * scale,
    y: (height - bounds.h * scale) / 2 - bounds.y * scale,
  };
}

/** Centre the world point `p` in the viewport at the current scale. */
export function centerOn(cam: Camera, p: Point, width: number, height: number): Camera {
  return { scale: cam.scale, x: width / 2 - p.x * cam.scale, y: height / 2 - p.y * cam.scale };
}

export function isCamera(v: unknown): v is Camera {
  const c = v as Camera;
  return !!c && [c.x, c.y, c.scale].every((n) => typeof n === 'number' && Number.isFinite(n)) && c.scale > 0;
}
