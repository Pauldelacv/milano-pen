import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useEffect, useState } from 'react';
import { Canvas } from '../canvas/Canvas';
import { clientToWorld } from '../canvas/viewport';
import { ContextMenu } from '../components/ContextMenu';
import { ImageViewer } from '../components/ImageViewer';
import { SearchDialog } from '../components/SearchDialog';
import { SettingsDialog } from '../components/SettingsDialog';
import { ShortcutsDialog } from '../components/ShortcutsDialog';
import { Toasts } from '../components/Toasts';
import { Toolbar } from '../components/Toolbar';
import { TopBar } from '../components/TopBar';
import { TrashDialog } from '../components/TrashDialog';
import { useTheme } from '../hooks/useTheme';
import { isUsefulDrag } from '../services/dropData';
import { api } from '../services/ipc';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';
import type { Settings } from '../types/models';
import * as A from './actions';
import { handleKeyDown, runCommand } from './commands';

function isTypingTarget(): boolean {
  const a = document.activeElement as HTMLElement | null;
  return !!a && (a.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(a.tagName));
}

export function App() {
  const [fatal, setFatal] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const dialog = useUiStore((s) => s.dialog);
  const boardGone = useBoardStore((s) => s.boardGone);
  useTheme();

  // Boot: settings, then the last board (or Home).
  useEffect(() => {
    if (!isTauri()) {
      setFatal('Milano Pen runs as a desktop app. Start it with “npm run app:dev”, not in a browser.');
      return;
    }
    (async () => {
      try {
        const [settings, project] = await Promise.all([api.getSettings(), api.projectInfo()]);
        useUiStore.getState().setSettings(settings);
        useUiStore.getState().setProject(project);
        await A.openInitialBoard();
        await A.refreshHistory();
        setReady(true);
      } catch (e) {
        setFatal(e instanceof Error ? e.message : String(e));
      }
    })();
  }, []);

  // Native menu, close/quit handshakes, restored backups, settings changed in
  // another window. Listeners are scoped to this window: events for other
  // project windows never reach it.
  useEffect(() => {
    if (!isTauri()) return;
    const win = getCurrentWebviewWindow();
    const flushThen = (done: () => Promise<void>) => async () => {
      try {
        await A.saveQueue.flush();
      } finally {
        await done();
      }
    };
    const offs = [
      win.listen<string>('menu', (e) => runCommand(e.payload, 'menu')),
      win.listen('app-will-quit', flushThen(api.readyToQuit)),
      win.listen('window-will-close', flushThen(api.readyToClose)),
      win.listen('workspace-replaced', () => void A.goHome()),
      win.listen<Settings>('settings-changed', (e) => useUiStore.getState().setSettings(e.payload)),
    ];
    return () => offs.forEach((p) => void p.then((off) => off()));
  }, []);

  // Drag and drop from Finder, browsers and other apps (HTML5 events: the
  // webview's own drop handling is what gives us browser images and links).
  useEffect(() => {
    let depth = 0;
    const editable = (t: EventTarget | null) =>
      t instanceof HTMLElement && !!t.closest('[contenteditable="true"], input, textarea, select');
    const accept = (e: DragEvent) =>
      !!e.dataTransfer && isUsefulDrag(e.dataTransfer.types) && !editable(e.target) && !useUiStore.getState().dialog;
    const onEnter = (e: DragEvent) => {
      if (!accept(e)) return;
      e.preventDefault();
      depth++;
      useUiStore.getState().setDropActive(true);
    };
    const onOver = (e: DragEvent) => {
      if (!accept(e)) return;
      e.preventDefault();
      e.dataTransfer!.dropEffect = 'copy';
    };
    const onLeave = () => {
      depth = Math.max(0, depth - 1);
      if (!depth) useUiStore.getState().setDropActive(false);
    };
    const onDrop = (e: DragEvent) => {
      depth = 0;
      useUiStore.getState().setDropActive(false);
      if (e.defaultPrevented || !accept(e)) return;
      e.preventDefault();
      void A.importDrop(e.dataTransfer!, clientToWorld(e.clientX, e.clientY));
    };
    window.addEventListener('dragenter', onEnter);
    window.addEventListener('dragover', onOver);
    window.addEventListener('dragleave', onLeave);
    window.addEventListener('drop', onDrop);
    return () => {
      window.removeEventListener('dragenter', onEnter);
      window.removeEventListener('dragover', onOver);
      window.removeEventListener('dragleave', onLeave);
      window.removeEventListener('drop', onDrop);
    };
  }, []);

  // Keyboard, clipboard, and flushing saves when the window loses focus.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => void handleKeyDown(e);
    const onCopy = (e: ClipboardEvent) => {
      if (!isTypingTarget() && !useUiStore.getState().dialog) void A.copySelection(e);
    };
    const onCut = (e: ClipboardEvent) => {
      if (!isTypingTarget() && !useUiStore.getState().dialog) void A.copySelection(e, true);
    };
    const onPaste = (e: ClipboardEvent) => {
      if (!isTypingTarget() && !useUiStore.getState().dialog) void A.pasteFromEvent(e);
    };
    const onBlur = () => void A.saveQueue.flush();
    const onVisibility = () => document.visibilityState === 'hidden' && void A.saveQueue.flush();
    window.addEventListener('keydown', onKey);
    document.addEventListener('copy', onCopy);
    document.addEventListener('cut', onCut);
    document.addEventListener('paste', onPaste);
    window.addEventListener('blur', onBlur);
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      window.removeEventListener('keydown', onKey);
      document.removeEventListener('copy', onCopy);
      document.removeEventListener('cut', onCut);
      document.removeEventListener('paste', onPaste);
      window.removeEventListener('blur', onBlur);
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, []);

  useEffect(() => {
    if (boardGone) void A.leaveDeletedBoard();
  }, [boardGone]);

  if (fatal) {
    return (
      <div className="fatal">
        <h1>Milano Pen couldn’t start</h1>
        <p>{fatal}</p>
      </div>
    );
  }

  return (
    <div className={`app${ready ? '' : ' booting'}`}>
      <TopBar />
      <main className="workspace">
        <Toolbar />
        <Canvas />
      </main>
      <ContextMenu />
      {dialog === 'search' && <SearchDialog />}
      {dialog === 'trash' && <TrashDialog />}
      {dialog === 'settings' && <SettingsDialog />}
      {dialog === 'shortcuts' && <ShortcutsDialog />}
      <ImageViewer />
      <Toasts />
    </div>
  );
}
