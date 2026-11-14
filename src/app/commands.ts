// Named commands, shared by the native menu and keyboard shortcuts. A
// command run from the keyboard is ignored if the same command then arrives
// from the menu (macOS can deliver both for one key press).
import { api } from '../services/ipc';
import { useBoardStore } from '../stores/boardStore';
import { selectedIds, useUiStore } from '../stores/uiStore';
import * as A from './actions';

export type CommandId =
  | 'new-note' | 'new-board' | 'add-image' | 'add-file' | 'import-json' | 'export-markdown' | 'export-json'
  | 'backup' | 'restore' | 'undo' | 'redo' | 'duplicate' | 'delete' | 'bring-to-front' | 'send-to-back'
  | 'search' | 'trash' | 'zoom-in' | 'zoom-out' | 'zoom-reset' | 'zoom-fit' | 'go-home' | 'go-up'
  | 'settings' | 'shortcuts' | 'open-data-folder' | 'benchmark' | 'website'
  | 'new-project' | 'open-project' | 'open-library' | 'print';

const lastRun = new Map<string, number>();

function isTypingTarget(): boolean {
  const a = document.activeElement as HTMLElement | null;
  return !!a && (a.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(a.tagName));
}

export function runCommand(id: string, source: 'menu' | 'keyboard' | 'ui' = 'ui'): void {
  const now = performance.now();
  if (source === 'menu' && now - (lastRun.get(id) ?? -1e9) < 250) return;
  lastRun.set(id, now);
  const ui = useUiStore.getState();
  // Text editing keeps the native behaviour of undo/redo.
  if ((id === 'undo' || id === 'redo') && isTypingTarget()) {
    if (source === 'menu') document.execCommand(id);
    return;
  }
  switch (id as CommandId) {
    case 'new-note': return void A.addNote();
    case 'new-board': return void A.addBoard();
    case 'add-image': return void A.addFiles(true);
    case 'add-file': return void A.addFiles(false);
    case 'import-json': return void A.importJson();
    case 'export-markdown': return void A.exportMarkdown();
    case 'export-json': return void A.exportJson();
    case 'backup': return void A.backupDatabase();
    case 'restore': return void A.restoreDatabase();
    case 'undo': return void A.undo();
    case 'redo': return void A.redo();
    case 'duplicate': return void A.duplicateSelection();
    case 'delete': return void A.deleteSelection();
    case 'bring-to-front': return void A.restack(true);
    case 'send-to-back': return void A.restack(false);
    case 'search': return ui.setDialog('search');
    case 'trash': return ui.setDialog('trash');
    case 'settings': return ui.setDialog('settings');
    case 'shortcuts': return ui.setDialog('shortcuts');
    case 'zoom-in': return A.zoomIn();
    case 'zoom-out': return A.zoomOut();
    case 'zoom-reset': return A.zoomReset();
    case 'zoom-fit': return A.zoomToFit(selectedIds().length > 1 ? selectedIds() : undefined);
    case 'go-home': return void A.goHome();
    case 'go-up': return void A.goUp();
    case 'open-data-folder': return void A.run(api.openDataFolder());
    case 'benchmark': return void A.generateBenchmark();
    case 'website': return void A.openLink('https://github.com/pauldelacv/milano-pen');
    case 'new-project': return void A.newProject();
    case 'open-project': return void A.openProject();
    case 'open-library': return void A.openLibrary();
    case 'print': return void A.printBoard();
  }
}

/** Global keyboard shortcuts. Returns true if the event was handled. */
export function handleKeyDown(e: KeyboardEvent): boolean {
  const ui = useUiStore.getState();
  if (e.defaultPrevented || ui.dialog || ui.viewerId) return false; // dialogs handle their own keys
  const mod = e.metaKey || e.ctrlKey;
  const typing = isTypingTarget();
  const key = e.key.toLowerCase();
  const cmd = (id: string) => {
    e.preventDefault();
    runCommand(id, 'keyboard');
    return true;
  };

  // Shortcuts that work everywhere, even while typing.
  if (mod && key === 'k') return cmd('search');
  if (mod && e.key === ',') return cmd('settings');
  if (mod && (e.key === '=' || e.key === '+')) return cmd('zoom-in');
  if (mod && e.key === '-') return cmd('zoom-out');
  if (mod && e.key === '0') return cmd('zoom-reset');
  if (mod && e.key === '1') return cmd('zoom-fit');
  if (typing) return false;

  if (mod && key === 'z') return cmd(e.shiftKey ? 'redo' : 'undo');
  if (mod && key === 'y') return cmd('redo');
  if (mod && key === 'd') return cmd('duplicate');
  if (mod && e.altKey && e.code === 'KeyN') return cmd('new-project');
  if (mod && key === 'n') return cmd(e.shiftKey ? 'new-board' : 'new-note');
  if (mod && key === 'o') return cmd('open-project');
  if (mod && key === 'p') return cmd('print');
  if (mod && e.shiftKey && key === 'i') return cmd('add-image');
  if (mod && e.shiftKey && key === 'h') return cmd('go-home');
  if (mod && e.key === 'ArrowUp') return cmd('go-up');
  if (mod && e.key === '/') return cmd('shortcuts');
  if (mod && (e.code === 'BracketRight' || e.code === 'BracketLeft')) return cmd(e.code === 'BracketRight' ? 'bring-to-front' : 'send-to-back');
  if (mod && key === 'a') {
    e.preventDefault();
    A.selectAll();
    return true;
  }
  if (mod) return false;

  const sel = selectedIds();
  if (e.key === 'Escape') {
    ui.setContextMenu(null);
    if (ui.editingId) ui.setEditing(null);
    else ui.clearSelection();
    return true;
  }
  if ((e.key === 'Backspace' || e.key === 'Delete') && sel.length) return cmd('delete');
  if (e.key === 'Enter' && sel.length === 1 && useBoardStore.getState().cards[sel[0]]) {
    e.preventDefault();
    void A.activateCard(sel[0]);
    return true;
  }
  const arrows: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
  if (arrows[e.key] && sel.length) {
    e.preventDefault();
    const step = e.shiftKey ? 10 : 1;
    A.nudge(arrows[e.key][0] * step, arrows[e.key][1] * step);
    return true;
  }
  return false;
}
