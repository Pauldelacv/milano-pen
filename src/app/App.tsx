import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
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
import { api } from '../services/ipc';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';
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
        useUiStore.getState().setSettings(await api.getSettings());
        await A.openInitialBoard();
        await A.refreshHistory();
        setReady(true);
      } catch (e) {
        setFatal(e instanceof Error ? e.message : String(e));
      }
    })();
  }, []);

  // Native menu, quit handshake, Finder drops, restored backups.
  useEffect(() => {
    if (!isTauri()) return;
    const offs = [
      listen<string>('menu', (e) => runCommand(e.payload, 'menu')),
      listen('app-will-quit', async () => {
        try {
          await A.saveQueue.flush();
        } finally {
          await api.readyToQuit();
        }
      }),
      listen<{ paths: string[]; x: number; y: number }>('files-dropped', (e) => {
        useUiStore.getState().setDropActive(false);
        void A.importDropped(e.payload.paths, clientToWorld(e.payload.x, e.payload.y));
      }),
      listen<{ active: boolean }>('files-drag', (e) => useUiStore.getState().setDropActive(e.payload.active)),
      listen('workspace-replaced', () => void A.goHome()),
    ];
    return () => offs.forEach((p) => void p.then((off) => off()));
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
