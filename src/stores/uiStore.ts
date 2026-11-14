import { create } from 'zustand';
import type { HistoryState, Settings } from '../types/models';

export type DialogName = 'search' | 'trash' | 'settings' | 'shortcuts' | null;

export interface Toast {
  id: number;
  message: string;
  kind: 'info' | 'error';
  action?: { label: string; run: () => void };
}

export interface ContextMenuState {
  x: number;
  y: number;
  /** World position the menu was opened at (for "Add … here"). */
  world: { x: number; y: number };
  /** Cards the menu acts on; empty for the canvas menu. */
  cardIds: string[];
}

export const DEFAULT_SETTINGS: Settings = {
  theme: 'system',
  wheelZooms: false,
  snapToGrid: true,
  gridSize: 10,
  autoBackup: true,
};

interface UiState {
  selection: Record<string, true>;
  editingId: string | null;
  dialog: DialogName;
  contextMenu: ContextMenuState | null;
  toasts: Toast[];
  settings: Settings;
  history: HistoryState;
  dropActive: boolean;
  saving: boolean;
  /** Image card shown in the full-screen viewer. */
  viewerId: string | null;
  setViewer: (id: string | null) => void;
  select: (ids: string[], additive?: boolean) => void;
  toggle: (id: string) => void;
  clearSelection: () => void;
  setEditing: (id: string | null) => void;
  setDialog: (d: DialogName) => void;
  setContextMenu: (m: ContextMenuState | null) => void;
  toast: (message: string, opts?: Partial<Omit<Toast, 'id' | 'message'>>) => void;
  dismissToast: (id: number) => void;
  setSettings: (s: Settings) => void;
  setHistory: (h: HistoryState) => void;
  setDropActive: (v: boolean) => void;
  setSaving: (v: boolean) => void;
}

let toastSeq = 0;

export const useUiStore = create<UiState>((set, get) => ({
  selection: {},
  editingId: null,
  dialog: null,
  contextMenu: null,
  toasts: [],
  settings: DEFAULT_SETTINGS,
  history: { canUndo: false, canRedo: false, undoLabel: null, redoLabel: null },
  dropActive: false,
  saving: false,
  viewerId: null,
  setViewer: (viewerId) => set({ viewerId, contextMenu: null }),

  select: (ids, additive = false) =>
    set((s) => {
      const selection: Record<string, true> = additive ? { ...s.selection } : {};
      for (const id of ids) selection[id] = true;
      return { selection };
    }),
  toggle: (id) =>
    set((s) => {
      const selection = { ...s.selection };
      if (selection[id]) delete selection[id];
      else selection[id] = true;
      return { selection };
    }),
  clearSelection: () => set((s) => (Object.keys(s.selection).length ? { selection: {} } : s)),
  setEditing: (editingId) => set({ editingId }),
  setDialog: (dialog) => set({ dialog, contextMenu: null }),
  setContextMenu: (contextMenu) => set({ contextMenu }),
  toast: (message, opts = {}) => {
    const id = ++toastSeq;
    set((s) => ({ toasts: [...s.toasts.slice(-3), { id, message, kind: opts.kind ?? 'info', action: opts.action }] }));
    setTimeout(() => get().dismissToast(id), opts.kind === 'error' ? 7000 : 4500);
  },
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
  setSettings: (settings) => set({ settings }),
  setHistory: (history) => set({ history }),
  setDropActive: (dropActive) => set({ dropActive }),
  setSaving: (saving) => set({ saving }),
}));

export const selectedIds = () => Object.keys(useUiStore.getState().selection);
