// The camera lives outside React: pointer and wheel handlers mutate it and a
// single requestAnimationFrame applies the CSS transform and notifies
// listeners (culling, zoom display). React re-renders only when something
// it shows actually changes.
import { create } from 'zustand';
import { DEFAULT_CAMERA, isCamera, type Camera } from './camera';

type Listener = (cam: Camera) => void;

let camera: Camera = { ...DEFAULT_CAMERA };
let world: HTMLElement | null = null;
let frame = 0;
let boardId: string | null = null;
let saveTimer: ReturnType<typeof setTimeout> | null = null;
const listeners = new Set<Listener>();

/** Zoom level for UI that depends on it, quantised to avoid churn. */
export const useZoom = create<{ scale: number; bucket: number }>(() => ({ scale: 1, bucket: 1 }));

const bucketOf = (s: number) => (s <= 0.35 ? 0.25 : s <= 0.75 ? 0.5 : s <= 1.5 ? 1 : s <= 3 ? 2 : 4);

function flushFrame() {
  frame = 0;
  if (world) world.style.transform = `translate3d(${camera.x}px, ${camera.y}px, 0) scale(${camera.scale})`;
  const z = useZoom.getState();
  if (z.scale !== camera.scale) useZoom.setState({ scale: camera.scale, bucket: bucketOf(camera.scale) });
  for (const l of listeners) l(camera);
  scheduleSave();
}

export function getCamera(): Camera {
  return camera;
}

export function setCamera(next: Camera): void {
  camera = next;
  if (!frame) frame = requestAnimationFrame(flushFrame);
}

export function attachWorld(el: HTMLElement | null): void {
  world = el;
  if (el) flushFrame();
}

export function onCamera(l: Listener): () => void {
  listeners.add(l);
  return () => listeners.delete(l);
}

// Per-board camera, remembered in localStorage (view state, not content).
const key = (id: string) => `milano-pen:camera:${id}`;

function scheduleSave() {
  if (!boardId) return;
  if (saveTimer) clearTimeout(saveTimer);
  const id = boardId;
  const cam = camera;
  saveTimer = setTimeout(() => {
    try {
      localStorage.setItem(key(id), JSON.stringify(cam));
    } catch {
      /* storage unavailable: the camera just isn't remembered */
    }
  }, 300);
}

/** Switch to a board's remembered camera (or the default). */
export function switchBoardCamera(id: string): void {
  boardId = id;
  let saved: unknown = null;
  try {
    saved = JSON.parse(localStorage.getItem(key(id)) ?? 'null');
  } catch {
    saved = null;
  }
  setCamera(isCamera(saved) ? saved : { ...DEFAULT_CAMERA });
}
