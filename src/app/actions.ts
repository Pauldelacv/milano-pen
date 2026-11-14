// Every user-facing operation, shared by the keyboard, the native menu, the
// toolbar and the context menu. UI components call these; these call the
// Rust backend through `api` and apply the returned patch.
import { centerOn, fitRect, zoomAt } from '../canvas/camera';
import { getCamera, setCamera, switchBoardCamera } from '../canvas/cameraController';
import { union } from '../canvas/geometry';
import { cardRect, isCanvasCard } from '../canvas/measure';
import { pointerOrCenterWorld, stageRect, viewportCenterWorld } from '../canvas/viewport';
import { parseColor } from '../cards/colorParse';
import { isUrl } from '../cards/embeds';
import { textToDoc } from '../editors/richText';
import { dataUrlToFile, readDrop, type DataTransferLike } from '../services/dropData';
import { api, BackendError } from '../services/ipc';
import { SaveQueue } from '../services/saveQueue';
import { setDataPinnedPredicate, useBoardStore } from '../stores/boardStore';
import { selectedIds, useUiStore } from '../stores/uiStore';
import type { CardKind, ImportResult, NewCard, Patch } from '../types/models';

const ui = () => useUiStore.getState();
const board = () => useBoardStore.getState();

export function reportError(e: unknown): void {
  const message = e instanceof BackendError || e instanceof Error ? e.message : String(e);
  ui().toast(message, { kind: 'error' });
  console.error(e);
}

/** Await `p`, turning failures into an error toast. */
export async function run<T>(p: Promise<T>): Promise<T | undefined> {
  try {
    return await p;
  } catch (e) {
    reportError(e);
    return undefined;
  }
}

export async function refreshHistory(): Promise<void> {
  const h = await run(api.historyState());
  if (h) ui().setHistory(h);
}

export function applyPatch(patch: Patch | null | undefined, opts: { select?: boolean } = {}): void {
  if (!patch) return;
  board().applyPatch(patch);
  if (opts.select && patch.created.length) ui().select(patch.created.filter((id) => board().cards[id]));
  void refreshHistory();
}

export const saveQueue = new SaveQueue({
  delayMs: 400,
  send: (updates, coalesce, label) => api.updateCards(updates, label, coalesce),
  onPatch: (p) => applyPatch(p),
  onError: (e) => {
    reportError(e);
    const id = board().boardId;
    if (id) void reloadBoard(id);
  },
  onBusyChange: (busy) => ui().setSaving(busy),
});
setDataPinnedPredicate((id) => saveQueue.isPending(id));

// ---- Navigation --------------------------------------------------------------

/** Remembered per project: every window shares this origin's localStorage. */
function lastBoardKey(): string {
  const p = ui().project;
  return !p || p.isLibrary ? 'milano-pen:last-board' : `milano-pen:last-board:${p.path}`;
}

async function reloadBoard(id: string): Promise<void> {
  const view = await run(api.openBoard(id));
  if (view) board().load(view);
}

export async function openBoard(id: string): Promise<boolean> {
  await saveQueue.flush();
  const view = await run(api.openBoard(id));
  if (!view) return false;
  ui().setEditing(null);
  ui().clearSelection();
  ui().setContextMenu(null);
  board().load(view);
  switchBoardCamera(id);
  try {
    localStorage.setItem(lastBoardKey(), id);
  } catch {
    /* not remembered */
  }
  return true;
}

export async function openInitialBoard(): Promise<void> {
  let last: string | null = null;
  try {
    last = localStorage.getItem(lastBoardKey());
  } catch {
    last = null;
  }
  if (last) {
    const view = await api.openBoard(last).catch(() => null);
    if (view) {
      board().load(view);
      switchBoardCamera(last);
      return;
    }
  }
  await openBoard(await api.rootBoard());
}

export async function goHome(): Promise<void> {
  const root = await run(api.rootBoard());
  if (root) await openBoard(root);
}

export async function goUp(): Promise<void> {
  const crumbs = board().breadcrumb;
  if (crumbs.length > 1) await openBoard(crumbs[crumbs.length - 2].id);
}

/** The open board was deleted (e.g. by undo): fall back to its nearest live ancestor. */
export async function leaveDeletedBoard(): Promise<void> {
  for (const c of [...board().breadcrumb].reverse()) {
    if (c.id !== board().boardId && (await api.openBoard(c.id).catch(() => null))) return void openBoard(c.id);
  }
  await goHome();
}

// ---- Creating cards ----------------------------------------------------------

const DEFAULT_SIZE: Partial<Record<CardKind, { w: number; h?: number }>> = {
  note: { w: 240 },
  board: { w: 140 },
  link: { w: 280 },
  swatch: { w: 140 },
};

export async function createCards(cards: NewCard[], opts: { edit?: boolean } = {}): Promise<Patch | undefined> {
  const patch = await run(api.createCards(cards));
  if (!patch) return undefined;
  applyPatch(patch, { select: true });
  if (opts.edit && patch.created.length === 1) ui().setEditing(patch.created[0]);
  return patch;
}

function centered(kind: CardKind, at = pointerOrCenterWorld()) {
  const w = DEFAULT_SIZE[kind]?.w ?? 240;
  return { x: Math.round(at.x - w / 2), y: Math.round(at.y - 30) };
}

export async function addCard(kind: CardKind, at?: { x: number; y: number }, data?: Record<string, unknown>) {
  const boardId = board().boardId;
  if (!boardId) return;
  const pos = at ?? centered(kind);
  const edit = kind === 'note' || kind === 'board';
  return createCards([{ boardId, kind, ...pos, w: DEFAULT_SIZE[kind]?.w, data }], { edit });
}

export const addNote = (at?: { x: number; y: number }) => addCard('note', at, { text: '', doc: textToDoc('') });
export const addBoard = (at?: { x: number; y: number }) => addCard('board', at, { title: 'Untitled board' });

export async function addLink(url: string, at?: { x: number; y: number }) {
  const clean = url.trim();
  if (!isUrl(clean)) {
    ui().toast('That doesn’t look like a web address (it should start with http:// or https://).', { kind: 'error' });
    return;
  }
  const boardId = board().boardId;
  if (!boardId) return;
  return createCards([{ boardId, kind: 'link', ...(at ?? centered('link')), w: 280, data: { url: clean, title: '' } }]);
}

/** A colour swatch from any CSS colour notation (normalised to #RRGGBB). */
export async function addSwatch(input: string, at?: { x: number; y: number }) {
  const hex = parseColor(input);
  if (!hex) {
    ui().toast('That isn’t a colour. Try a hex code like #4C6EF5, rgb(76, 110, 245) or a name like “teal”.', { kind: 'error' });
    return;
  }
  const boardId = board().boardId;
  if (!boardId) return;
  return createCards([{ boardId, kind: 'swatch', ...(at ?? centered('swatch')), w: DEFAULT_SIZE.swatch?.w, data: { hex } }]);
}

export async function addTextAsNote(text: string, at?: { x: number; y: number }) {
  const boardId = board().boardId;
  if (!boardId) return;
  const t = text.replace(/\r\n?/g, '\n').slice(0, 100_000);
  return createCards([{ boardId, kind: 'note', ...(at ?? centered('note')), data: { text: t, doc: textToDoc(t) } }]);
}

function reportImport(r: ImportResult | null | undefined) {
  if (!r) return;
  applyPatch(r.patch, { select: true });
  if (r.errors.length) ui().toast(`Some files were not added: ${r.errors.join('; ')}`, { kind: 'error' });
}

export async function addFiles(imagesOnly: boolean, at?: { x: number; y: number }) {
  const boardId = board().boardId;
  if (!boardId) return;
  const p = at ?? pointerOrCenterWorld();
  reportImport(await run(api.pickAndImportFiles(boardId, [p.x, p.y], imagesOnly)));
}

export async function importClipboardFiles(files: File[], at: { x: number; y: number }) {
  const boardId = board().boardId;
  if (!boardId) return;
  let x = at.x;
  for (const f of files) {
    reportImport(await run(api.importClipboardFile(boardId, f, [x, at.y])));
    x += 300;
  }
}

/**
 * A drop from Finder, a browser or another app: files (including images
 * dragged out of a web page), inline images, links, then plain text.
 * Nothing is downloaded: a link without a file becomes a link card.
 */
export async function importDrop(dt: DataTransferLike, at: { x: number; y: number }) {
  const drop = readDrop(dt);
  if (drop.files.length) return importClipboardFiles(drop.files, at);
  const inline = drop.dataImages.map((u, i) => dataUrlToFile(u, `dropped-${i + 1}`)).filter((f): f is File => !!f);
  if (inline.length) return importClipboardFiles(inline, at);
  if (drop.urls.length) {
    let y = at.y;
    for (const url of drop.urls.slice(0, 50)) {
      await addLink(url, { x: Math.round(at.x - 140), y: Math.round(y - 30) });
      y += 90;
    }
    return;
  }
  if (drop.text.trim()) return addTextAsNote(drop.text, { x: Math.round(at.x - 120), y: Math.round(at.y - 20) });
}

// ---- Editing the selection ---------------------------------------------------

export async function deleteCards(ids: string[]) {
  if (!ids.length) return;
  await saveQueue.flush();
  const patch = await run(api.trashCards(ids));
  if (!patch) return;
  ui().setEditing(null);
  ui().clearSelection();
  applyPatch(patch);
  ui().toast(ids.length === 1 ? 'Moved to Trash' : `${ids.length} cards moved to Trash`, {
    action: { label: 'Undo', run: () => void undo() },
  });
}

export const deleteSelection = () => deleteCards(selectedIds());

export async function duplicateSelection() {
  const ids = selectedIds();
  if (!ids.length) return;
  await saveQueue.flush();
  applyPatch(await run(api.duplicate(ids)), { select: true });
}

export async function restack(toFront: boolean, ids = selectedIds()) {
  if (ids.length) applyPatch(await run(api.restack(ids, toFront)));
}

export async function setColor(ids: string[], color: string | null) {
  if (!ids.length) return;
  applyPatch(await run(api.updateCards(ids.map((id) => ({ id, color: color ?? '' })), 'Change colour')));
}

export async function updateData(id: string, data: Record<string, unknown>, label = 'Edit') {
  board().setLocal([{ id, data }]);
  applyPatch(await run(api.updateCards([{ id, data }], label)));
}

export function selectAll() {
  ui().select(Object.values(board().cards).filter(isCanvasCard).map((c) => c.id));
}

export function nudge(dx: number, dy: number) {
  const ids = selectedIds();
  const cards = board().cards;
  const changes = ids.filter((id) => cards[id]).map((id) => ({ id, x: cards[id].x + dx, y: cards[id].y + dy }));
  if (!changes.length) return;
  board().setLocal(changes);
  for (const c of changes) saveQueue.schedule(c, 'Move', `nudge:${ids.join(',')}`);
}

// ---- Clipboard -----------------------------------------------------------------

/** Marker MIME type for cards copied inside Milano Pen. */
const CARDS_MIME = 'application/x-milano-pen-cards';
let lastCopiedText: string | null = null;

function plainTextOf(ids: string[]): string {
  const cards = board().cards;
  return ids
    .map((id) => cards[id])
    .filter(Boolean)
    .map((c) => String(c.data.text ?? c.data.title ?? c.data.url ?? c.data.name ?? c.data.hex ?? ''))
    .filter(Boolean)
    .join('\n\n');
}

export async function copySelection(e: ClipboardEvent | null, cut = false): Promise<boolean> {
  const ids = selectedIds();
  if (!ids.length) return false;
  await saveQueue.flush();
  const text = plainTextOf(ids) || `${ids.length} Milano Pen card${ids.length > 1 ? 's' : ''}`;
  if (e?.clipboardData) {
    e.preventDefault();
    e.clipboardData.setData('text/plain', text);
    e.clipboardData.setData(CARDS_MIME, String(ids.length));
  }
  lastCopiedText = text;
  const n = await run(api.copyCards(ids));
  if (!n) return false;
  if (cut) await deleteCards(ids);
  return true;
}

export async function pasteCards(at = pointerOrCenterWorld()) {
  const boardId = board().boardId;
  if (!boardId) return;
  applyPatch(await run(api.pasteCards(boardId, [Math.round(at.x), Math.round(at.y)])), { select: true });
}

/** Paste from the system clipboard: our cards, images/files, URLs or text. */
export async function pasteFromEvent(e: ClipboardEvent) {
  const dt = e.clipboardData;
  if (!dt) return;
  const at = pointerOrCenterWorld();
  const text = dt.getData('text/plain');
  if (dt.types.includes(CARDS_MIME) || (lastCopiedText !== null && text === lastCopiedText)) {
    e.preventDefault();
    return pasteCards(at);
  }
  const files = [...dt.files];
  if (files.length) {
    e.preventDefault();
    return importClipboardFiles(files, at);
  }
  if (text.trim()) {
    e.preventDefault();
    return isUrl(text) ? addLink(text, at) : addTextAsNote(text, at);
  }
}

// ---- History -------------------------------------------------------------------

export async function undo() {
  await saveQueue.flush();
  ui().setEditing(null);
  const patch = await run(api.undo());
  if (patch) {
    applyPatch(patch);
    ui().toast(`Undid ${patch.label.toLowerCase()}`);
  }
}

export async function redo() {
  await saveQueue.flush();
  ui().setEditing(null);
  const patch = await run(api.redo());
  if (patch) {
    applyPatch(patch);
    ui().toast(`Redid ${patch.label.toLowerCase()}`);
  }
}

// ---- Camera ----------------------------------------------------------------------

function zoomCenter(factor: number) {
  const r = stageRect();
  setCamera(zoomAt(getCamera(), { x: r.width / 2, y: r.height / 2 }, factor));
}

export const zoomIn = () => zoomCenter(1.25);
export const zoomOut = () => zoomCenter(1 / 1.25);
export function zoomReset() {
  const r = stageRect();
  const c = viewportCenterWorld();
  setCamera(centerOn({ ...getCamera(), scale: 1 }, c, r.width, r.height));
}

export function zoomToFit(ids?: string[]) {
  const cards = Object.values(board().cards).filter((c) => isCanvasCard(c) && (!ids || ids.includes(c.id)));
  const bounds = union(cards.map(cardRect));
  const r = stageRect();
  if (!bounds) return setCamera({ x: 80, y: 60, scale: 1 });
  setCamera(fitRect(bounds, r.width, r.height));
}

export function centerOnCard(id: string) {
  const c = board().cards[id];
  if (!c) return;
  const rect = cardRect(c);
  const r = stageRect();
  setCamera(centerOn({ ...getCamera(), scale: Math.max(getCamera().scale, 0.6) }, { x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 }, r.width, r.height));
}

// ---- Files, exports, backups -------------------------------------------------------

export async function exportMarkdown() {
  const id = board().boardId;
  if (!id) return;
  const path = await run(api.exportMarkdown(id));
  if (path) ui().toast(`Exported to ${path}`);
}

export async function exportJson() {
  const id = board().boardId;
  if (!id) return;
  const path = await run(api.exportJson(id));
  if (path) ui().toast(`Exported to ${path}`);
}

export async function importJson() {
  const id = board().boardId;
  if (!id) return;
  const c = viewportCenterWorld();
  const patch = await run(api.importJson(id, [c.x, c.y]));
  if (patch) {
    applyPatch(patch, { select: true });
    ui().toast(`Imported ${patch.created.length} card${patch.created.length === 1 ? '' : 's'}`);
  }
}

export async function backupDatabase() {
  const path = await run(api.backupDatabase());
  if (path) ui().toast(`Backup saved to ${path}`);
}

export async function restoreDatabase() {
  await saveQueue.flush();
  const ok = await run(api.restoreDatabase());
  if (ok) {
    ui().toast('Backup restored. Your previous data was saved in the backups folder.');
    await goHome();
  }
}

export async function generateBenchmark() {
  const id = board().boardId;
  if (!id) return;
  ui().toast('Generating a 1,000-card benchmark board…');
  const patch = await run(api.generateBenchmark(id));
  if (patch) {
    applyPatch(patch, { select: true });
    ui().toast('Benchmark board ready. Open it to test performance.');
  }
}

// ---- Projects and printing -------------------------------------------------------

export const newProject = () => run(api.newProject());
export const openProject = () => run(api.openProject());
export const openLibrary = () => run(api.openLibrary());

/** Usable area of a landscape A4 or US Letter page (1 cm margins), in CSS px. */
const PRINT_PAGE = { w: 979, h: 718 };

/** Print the current view (File ▸ Print, ⌘P). The print stylesheet hides the
 * interface and the grid dots and scales the view to fit the page; the macOS
 * dialog can print it or save it as PDF. */
export async function printBoard() {
  await saveQueue.flush();
  ui().setEditing(null);
  ui().clearSelection();
  ui().setContextMenu(null);
  const r = stageRect();
  const scale = Math.min(1, PRINT_PAGE.w / r.width, PRINT_PAGE.h / r.height);
  const root = document.documentElement;
  root.style.setProperty('--print-scale', String(scale));
  root.style.setProperty('--print-width', `${r.width}px`);
  root.style.setProperty('--print-height', `${r.height}px`);
  const title = document.title;
  document.title = board().board?.title || title;
  // Let React drop the selection outlines before the page is captured.
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  try {
    await api.printBoard();
  } catch (e) {
    // Outside the app shell (or if the native dialog is unavailable).
    if (typeof window.print === 'function') window.print();
    else reportError(e);
  } finally {
    document.title = title;
  }
}

export async function openLink(url: string) {
  await run(api.openExternal(url));
}

/** Double-click / Enter on a card: its main action. */
export async function activateCard(id: string) {
  const c = board().cards[id];
  if (!c) return;
  switch (c.kind) {
    case 'board': {
      const child = c.data.childBoardId;
      if (typeof child === 'string') await openBoard(child);
      break;
    }
    case 'image':
      ui().setViewer(id);
      break;
    case 'file':
      if (typeof c.data.assetId === 'string') await run(api.openAsset(c.data.assetId));
      break;
    case 'link':
      if (typeof c.data.url === 'string') await openLink(c.data.url);
      break;
    case 'note':
    case 'swatch':
      ui().select([id]);
      ui().setEditing(id);
      break;
    default:
      break;
  }
}
