// The infinite canvas: camera, culling and every pointer interaction
// (select, marquee, drag, resize, pan, zoom). Cards themselves are dumb views.
import { useCallback, useEffect, useRef, useState } from 'react';
import { activateCard, addNote, applyPatch, run, saveQueue } from '../app/actions';
import { CardView } from '../cards/CardView';
import { api } from '../services/ipc';
import { useBoardStore, type LocalChange } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';
import { screenToWorld, visibleWorldRect, zoomAt } from './camera';
import { attachWorld, getCamera, onCamera, setCamera } from './cameraController';
import { expand, rectFromPoints, snap, type Point, type Rect } from './geometry';
import { cardRect, isCanvasCard, onMeasured } from './measure';
import { SpatialIndex } from './spatialIndex';
import { attachStage, setLastPointer } from './viewport';

/** Extra area mounted around the viewport, as a fraction of its size. */
const OVERSCAN = 0.5;
const DRAG_THRESHOLD = 3;
const MIN_CARD = 40;

type Interaction =
  | { kind: 'pan'; start: Point; cam: { x: number; y: number } }
  | { kind: 'marquee'; start: Point; additive: boolean; base: string[] }
  | { kind: 'drag'; start: Point; origins: Map<string, Point>; lead: string; moved: boolean }
  | { kind: 'resize'; id: string; start: Point; w: number; h: number | null; aspect: number | null };

const sameIds = (a: string[], b: string[]) => a.length === b.length && a.every((v, i) => v === b[i]);

export function Canvas() {
  const stageRef = useRef<HTMLDivElement>(null);
  const worldRef = useRef<HTMLDivElement>(null);
  const indexRef = useRef(new SpatialIndex());
  const indexedRef = useRef(new Map<string, unknown>());
  const interaction = useRef<Interaction | null>(null);
  const pendingMove = useRef<PointerEvent | null>(null);
  const moveFrame = useRef(0);
  const spaceHeld = useRef(false);
  const gesture = useRef<{ scale: number } | null>(null);

  const [visible, setVisible] = useState<string[]>([]);
  const [marquee, setMarquee] = useState<Rect | null>(null);
  const [panning, setPanning] = useState(false);
  const cards = useBoardStore((s) => s.cards);
  const boardId = useBoardStore((s) => s.boardId);
  const dropActive = useUiStore((s) => s.dropActive);

  // ---- Culling ---------------------------------------------------------------
  const recompute = useCallback(() => {
    const stage = stageRef.current;
    if (!stage) return;
    const cam = getCamera();
    const { width, height } = stage.getBoundingClientRect();
    const view = visibleWorldRect(cam, width, height);
    const ids = indexRef.current.query(expand(view, view.w * OVERSCAN, view.h * OVERSCAN)).sort();
    setVisible((prev) => (sameIds(prev, ids) ? prev : ids));
    stage.style.setProperty('--zoom', String(cam.scale));
    stage.style.backgroundPosition = `${cam.x}px ${cam.y}px`;
    stage.style.backgroundSize = `${20 * cam.scale}px ${20 * cam.scale}px`;
  }, []);

  // Keep the spatial index in step with the cards (only changed ones touched).
  const indexedBoard = useRef<string | null>(null);
  useEffect(() => {
    const idx = indexRef.current;
    const seen = indexedRef.current;
    if (indexedBoard.current !== boardId) {
      // Another board: start from an empty index.
      idx.clear();
      seen.clear();
      indexedBoard.current = boardId;
    }
    for (const id of [...seen.keys()]) {
      if (!cards[id] || !isCanvasCard(cards[id])) {
        idx.remove(id);
        seen.delete(id);
      }
    }
    for (const c of Object.values(cards)) {
      if (!isCanvasCard(c) || seen.get(c.id) === c) continue;
      idx.set(c.id, cardRect(c));
      seen.set(c.id, c);
    }
    recompute();
  }, [cards, boardId, recompute]);

  useEffect(() => {
    let frame = 0;
    const offMeasure = onMeasured((id) => {
      const c = useBoardStore.getState().cards[id];
      if (c && isCanvasCard(c)) indexRef.current.set(id, cardRect(c));
      if (!frame) frame = requestAnimationFrame(() => ((frame = 0), recompute()));
    });
    const offCam = onCamera(recompute);
    const ro = new ResizeObserver(recompute);
    if (stageRef.current) ro.observe(stageRef.current);
    return () => {
      offMeasure();
      offCam();
      ro.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [recompute]);

  useEffect(() => {
    attachStage(stageRef.current);
    attachWorld(worldRef.current);
    return () => {
      attachStage(null);
      attachWorld(null);
    };
  }, []);

  // ---- Helpers ----------------------------------------------------------------
  const toWorld = useCallback((clientX: number, clientY: number): Point => {
    const r = stageRef.current!.getBoundingClientRect();
    return screenToWorld(getCamera(), { x: clientX - r.left, y: clientY - r.top });
  }, []);

  const toScreen = useCallback((clientX: number, clientY: number): Point => {
    const r = stageRef.current!.getBoundingClientRect();
    return { x: clientX - r.left, y: clientY - r.top };
  }, []);

  // ---- Wheel, pinch, space ----------------------------------------------------
  useEffect(() => {
    const stage = stageRef.current!;
    const onWheel = (e: WheelEvent) => {
      if ((e.target as HTMLElement).closest('[data-scrollable]')) return;
      e.preventDefault();
      const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 400 : 1;
      const dx = e.deltaX * unit;
      const dy = e.deltaY * unit;
      const anchor = toScreen(e.clientX, e.clientY);
      const pinch = e.ctrlKey; // trackpad pinch arrives as ctrl+wheel
      if (pinch && gesture.current) return; // Safari also sends gesture events
      if (pinch || e.metaKey || useUiStore.getState().settings.wheelZooms) {
        const factor = Math.exp(-dy * (pinch ? 0.01 : 0.0015));
        setCamera(zoomAt(getCamera(), anchor, factor));
      } else {
        const cam = getCamera();
        setCamera({ ...cam, x: cam.x - (e.shiftKey && !dx ? dy : dx), y: cam.y - (e.shiftKey && !dx ? 0 : dy) });
      }
    };
    // WebKit trackpad pinch.
    type GestureEvent = UIEvent & { scale: number; clientX: number; clientY: number };
    const onGestureStart = (e: Event) => {
      e.preventDefault();
      gesture.current = { scale: 1 };
    };
    const onGestureChange = (e: Event) => {
      const g = e as GestureEvent;
      e.preventDefault();
      if (!gesture.current) return;
      const factor = g.scale / gesture.current.scale;
      gesture.current.scale = g.scale;
      setCamera(zoomAt(getCamera(), toScreen(g.clientX, g.clientY), factor));
    };
    const onGestureEnd = (e: Event) => {
      e.preventDefault();
      gesture.current = null;
    };
    stage.addEventListener('wheel', onWheel, { passive: false });
    stage.addEventListener('gesturestart', onGestureStart);
    stage.addEventListener('gesturechange', onGestureChange);
    stage.addEventListener('gestureend', onGestureEnd);

    const typing = () => {
      const a = document.activeElement as HTMLElement | null;
      return !!a && (a.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(a.tagName));
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.code === 'Space' && !typing() && !useUiStore.getState().dialog) {
        e.preventDefault();
        if (!spaceHeld.current) {
          spaceHeld.current = true;
          stage.classList.add('space-pan');
        }
      }
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.code === 'Space') {
        spaceHeld.current = false;
        stage.classList.remove('space-pan');
      }
    };
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('keyup', onKeyUp);
    return () => {
      stage.removeEventListener('wheel', onWheel);
      stage.removeEventListener('gesturestart', onGestureStart);
      stage.removeEventListener('gesturechange', onGestureChange);
      stage.removeEventListener('gestureend', onGestureEnd);
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('keyup', onKeyUp);
    };
  }, [toScreen]);

  // ---- Pointer interactions ------------------------------------------------------
  const processMove = useCallback(() => {
    moveFrame.current = 0;
    const e = pendingMove.current;
    const it = interaction.current;
    if (!e || !it) return;
    const ui = useUiStore.getState();
    const store = useBoardStore.getState();
    const { snapToGrid, gridSize } = ui.settings;
    switch (it.kind) {
      case 'pan': {
        setCamera({ ...getCamera(), x: it.cam.x + e.clientX - it.start.x, y: it.cam.y + e.clientY - it.start.y });
        break;
      }
      case 'marquee': {
        const p = toWorld(e.clientX, e.clientY);
        const rect = rectFromPoints(it.start, p);
        const cam = getCamera();
        setMarquee({ x: rect.x * cam.scale + cam.x, y: rect.y * cam.scale + cam.y, w: rect.w * cam.scale, h: rect.h * cam.scale });
        const hits = indexRef.current.query(rect);
        ui.select(it.additive ? [...new Set([...it.base, ...hits])] : hits);
        break;
      }
      case 'drag': {
        const p = toWorld(e.clientX, e.clientY);
        let dx = p.x - it.start.x;
        let dy = p.y - it.start.y;
        if (!it.moved) {
          const scale = getCamera().scale;
          if (Math.hypot(dx * scale, dy * scale) < DRAG_THRESHOLD) return;
          it.moved = true;
          ui.setContextMenu(null);
        }
        // Snap the card under the pointer; the others keep their offsets.
        const lead = it.origins.get(it.lead)!;
        const free = e.altKey || !snapToGrid;
        dx = snap(lead.x + dx, gridSize, !free) - lead.x;
        dy = snap(lead.y + dy, gridSize, !free) - lead.y;
        const changes: LocalChange[] = [];
        for (const [id, o] of it.origins) changes.push({ id, x: o.x + dx, y: o.y + dy });
        store.setLocal(changes);
        break;
      }
      case 'resize': {
        const p = toWorld(e.clientX, e.clientY);
        const free = e.altKey || !snapToGrid;
        const w = Math.max(MIN_CARD, snap(it.w + p.x - it.start.x, gridSize, !free));
        const change: LocalChange = { id: it.id, w };
        if (it.aspect) change.h = Math.round(w * it.aspect);
        else if (it.h != null) change.h = Math.max(MIN_CARD, snap(it.h + p.y - it.start.y, gridSize, !free));
        store.setLocal([change]);
        break;
      }
    }
  }, [toWorld]);

  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      setLastPointer(e.clientX, e.clientY);
      if (!interaction.current) return;
      pendingMove.current = e.nativeEvent;
      if (!moveFrame.current) moveFrame.current = requestAnimationFrame(processMove);
    },
    [processMove],
  );

  const endInteraction = useCallback(async () => {
    const it = interaction.current;
    interaction.current = null;
    if (moveFrame.current) {
      cancelAnimationFrame(moveFrame.current);
      processMove();
    }
    pendingMove.current = null;
    setPanning(false);
    setMarquee(null);
    if (!it) return;
    const cards = useBoardStore.getState().cards;
    if (it.kind === 'drag' && it.moved) {
      const updates = [...it.origins.keys()].filter((id) => cards[id]).map((id) => ({ id, x: cards[id].x, y: cards[id].y }));
      // Persist the final position once, at the end of the interaction.
      const patch = await run(api.updateCards(updates, updates.length > 1 ? `Move ${updates.length} cards` : 'Move card'));
      if (patch) applyPatch(patch);
      else if (useBoardStore.getState().boardId) {
        const view = await run(api.openBoard(useBoardStore.getState().boardId!));
        if (view) useBoardStore.getState().load(view);
      }
    }
    if (it.kind === 'resize') {
      const c = cards[it.id];
      if (c && (c.w !== it.w || c.h !== it.h)) {
        applyPatch(await run(api.updateCards([{ id: c.id, w: c.w, ...(c.h != null && { h: c.h }) }], 'Resize card')));
      }
    }
  }, [processMove]);

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      const target = e.target as HTMLElement;
      const ui = useUiStore.getState();
      ui.setContextMenu(null);
      if (e.button === 2) return; // context menu
      const stage = stageRef.current!;
      const startPan = e.button === 1 || spaceHeld.current;
      if (startPan) {
        e.preventDefault();
        interaction.current = { kind: 'pan', start: { x: e.clientX, y: e.clientY }, cam: { ...getCamera() } };
        setPanning(true);
        stage.setPointerCapture(e.pointerId);
        return;
      }
      if (e.button !== 0) return;
      const cardEl = target.closest<HTMLElement>('[data-card-id]');
      if (!cardEl) {
        // Empty canvas: leave editing, start a marquee.
        if (ui.editingId) {
          ui.setEditing(null);
          void saveQueue.flush();
        }
        const additive = e.shiftKey || e.metaKey;
        if (!additive) ui.clearSelection();
        (document.activeElement as HTMLElement | null)?.blur?.();
        interaction.current = { kind: 'marquee', start: toWorld(e.clientX, e.clientY), additive, base: Object.keys(ui.selection) };
        stage.setPointerCapture(e.pointerId);
        return;
      }
      const id = cardEl.dataset.cardId!;
      if (ui.editingId === id) return; // text selection inside the editor
      if (target.closest('[data-no-drag]')) return;
      if (ui.editingId) {
        ui.setEditing(null);
        void saveQueue.flush();
      }
      const card = useBoardStore.getState().cards[id];
      if (!card) return;
      if (target.closest('[data-resize]')) {
        e.preventDefault();
        ui.select([id]);
        interaction.current = {
          kind: 'resize',
          id,
          start: toWorld(e.clientX, e.clientY),
          w: card.w,
          h: card.h,
          aspect: card.kind === 'image' && card.h ? card.h / card.w : null,
        };
        stage.setPointerCapture(e.pointerId);
        return;
      }
      e.preventDefault();
      (document.activeElement as HTMLElement | null)?.blur?.();
      if (e.shiftKey || e.metaKey) {
        ui.toggle(id);
        if (!useUiStore.getState().selection[id]) return;
      } else if (!ui.selection[id]) {
        ui.select([id]);
      }
      const cards = useBoardStore.getState().cards;
      const origins = new Map<string, Point>();
      for (const sid of Object.keys(useUiStore.getState().selection)) {
        const c = cards[sid];
        if (c && isCanvasCard(c)) origins.set(sid, { x: c.x, y: c.y });
      }
      interaction.current = { kind: 'drag', start: toWorld(e.clientX, e.clientY), origins, lead: id, moved: false };
      stage.setPointerCapture(e.pointerId);
    },
    [toWorld],
  );

  const onDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      // Pointer capture retargets clicks to the stage: hit-test instead.
      const target = (document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null) ?? (e.target as HTMLElement);
      const cardEl = target.closest<HTMLElement>('[data-card-id]');
      if (cardEl) {
        if (useUiStore.getState().editingId !== cardEl.dataset.cardId) void activateCard(cardEl.dataset.cardId!);
        return;
      }
      if (target.closest('[data-stage]')) {
        const p = toWorld(e.clientX, e.clientY);
        void addNote({ x: Math.round(p.x - 120), y: Math.round(p.y - 20) });
      }
    },
    [toWorld],
  );

  const onContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      const ui = useUiStore.getState();
      const cardEl = (e.target as HTMLElement).closest<HTMLElement>('[data-card-id]');
      if (cardEl && ui.editingId === cardEl.dataset.cardId) return;
      let ids: string[] = [];
      if (cardEl) {
        const id = cardEl.dataset.cardId!;
        if (!ui.selection[id]) ui.select([id]);
        ids = Object.keys(useUiStore.getState().selection);
      } else {
        ui.clearSelection();
      }
      ui.setContextMenu({ x: e.clientX, y: e.clientY, world: toWorld(e.clientX, e.clientY), cardIds: ids });
    },
    [toWorld],
  );

  return (
    <div
      ref={stageRef}
      className={`stage${panning ? ' panning' : ''}${dropActive ? ' drop-active' : ''}`}
      data-stage
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={endInteraction}
      onPointerCancel={endInteraction}
      onDoubleClick={onDoubleClick}
      onContextMenu={onContextMenu}
    >
      <div ref={worldRef} className="world">
        {visible.map((id) => (
          <CardView key={id} id={id} />
        ))}
      </div>
      {marquee && (
        <div className="marquee" style={{ left: marquee.x, top: marquee.y, width: marquee.w, height: marquee.h }} />
      )}
      {Object.keys(cards).length === 0 && boardId && (
        <div className="empty-hint">
          <p>This board is empty.</p>
          <p>Double-click anywhere to add a note, drop files from Finder, or use the toolbar on the left.</p>
        </div>
      )}
    </div>
  );
}
