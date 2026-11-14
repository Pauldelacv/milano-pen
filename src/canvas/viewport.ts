// Where the canvas is on screen and where the pointer last was, for actions
// triggered from menus and the keyboard.
import { getCamera } from './cameraController';
import { screenToWorld } from './camera';
import type { Point } from './geometry';

let stage: HTMLElement | null = null;
let lastPointer: Point | null = null;

export function attachStage(el: HTMLElement | null): void {
  stage = el;
}

export function stageRect(): DOMRect {
  return stage?.getBoundingClientRect() ?? new DOMRect(0, 0, window.innerWidth, window.innerHeight);
}

export function setLastPointer(clientX: number, clientY: number): void {
  lastPointer = { x: clientX, y: clientY };
}

export function clientToWorld(clientX: number, clientY: number): Point {
  const r = stageRect();
  return screenToWorld(getCamera(), { x: clientX - r.left, y: clientY - r.top });
}

export function viewportCenterWorld(): Point {
  const r = stageRect();
  return clientToWorld(r.left + r.width / 2, r.top + r.height / 2);
}

/** The pointer's world position if it is over the canvas, else the centre. */
export function pointerOrCenterWorld(): Point {
  const r = stageRect();
  if (lastPointer && lastPointer.x >= r.left && lastPointer.x <= r.right && lastPointer.y >= r.top && lastPointer.y <= r.bottom) {
    return clientToWorld(lastPointer.x, lastPointer.y);
  }
  return viewportCenterWorld();
}
