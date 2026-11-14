// Typed wrappers around the Rust commands. UI code talks to the backend only
// through this module.
import { invoke } from '@tauri-apps/api/core';
import type {
  AppInfo,
  BoardUpdate,
  BoardView,
  CardUpdate,
  HistoryState,
  ImportResult,
  IpcError,
  NewCard,
  Patch,
  ProjectInfo,
  SearchResult,
  Settings,
  TrashEntry,
} from '../types/models';

export class BackendError extends Error {
  kind: string;
  constructor(e: unknown) {
    const err = e as Partial<IpcError> | string | undefined;
    const message = typeof err === 'string' ? err : (err?.message ?? 'Something went wrong');
    super(message);
    this.kind = typeof err === 'object' && err?.kind ? err.kind : 'unknown';
  }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw new BackendError(e);
  }
}

type At = [number, number];

export const api = {
  rootBoard: () => call<string>('get_root_board'),
  openBoard: (id: string) => call<BoardView>('open_board', { id }),
  updateBoard: (id: string, update: BoardUpdate) => call<Patch>('update_board', { id, update }),

  createCards: (cards: NewCard[]) => call<Patch>('create_cards', { cards }),
  updateCards: (updates: CardUpdate[], label?: string, coalesce?: string) =>
    call<Patch>('update_cards', { updates, label, coalesce }),
  restack: (ids: string[], toFront: boolean) => call<Patch>('restack_cards', { ids, toFront }),
  trashCards: (ids: string[]) => call<Patch>('trash_cards', { ids }),
  duplicate: (ids: string[]) => call<Patch>('duplicate_cards', { ids }),
  copyCards: (ids: string[]) => call<number>('copy_cards', { ids }),
  pasteCards: (boardId: string, at: At | null) => call<Patch>('paste_cards', { boardId, at }),

  undo: () => call<Patch | null>('undo'),
  redo: () => call<Patch | null>('redo'),
  historyState: () => call<HistoryState>('history_state'),

  search: (query: string) => call<SearchResult[]>('search', { query }),
  listTrash: () => call<TrashEntry[]>('list_trash'),
  restoreTrashed: (id: string) => call<Patch>('restore_trashed', { id }),
  emptyTrash: () => call<{ cards: number; boards: number; files: number }>('empty_trash'),

  pickAndImportFiles: (boardId: string, at: At, imagesOnly: boolean) =>
    call<ImportResult | null>('pick_and_import_files', { boardId, at, imagesOnly }),
  importClipboardFile: async (boardId: string, file: File, at: At) => {
    const bytes = new Uint8Array(await file.arrayBuffer());
    try {
      return await invoke<ImportResult>('import_clipboard_file', bytes, {
        headers: {
          'x-board-id': boardId,
          'x-file-name': encodeURIComponent(file.name || 'pasted.png'),
          'x-at-x': String(at[0]),
          'x-at-y': String(at[1]),
        },
      });
    } catch (e) {
      throw new BackendError(e);
    }
  },
  openAsset: (assetId: string) => call<void>('open_asset', { assetId }),
  revealAsset: (assetId: string) => call<void>('reveal_asset', { assetId }),

  exportMarkdown: (boardId: string) => call<string | null>('export_board_markdown', { boardId }),
  exportJson: (boardId: string) => call<string | null>('export_board_json', { boardId }),
  importJson: (boardId: string, at: At) => call<Patch | null>('import_board_json', { boardId, at }),
  backupDatabase: () => call<string | null>('backup_database'),
  restoreDatabase: () => call<boolean>('restore_database'),

  getSettings: () => call<Settings>('get_settings'),
  saveSettings: (settings: Settings) => call<Settings>('save_settings', { settings }),
  appInfo: () => call<AppInfo>('app_info'),
  openExternal: (url: string) => call<void>('open_external', { url }),
  openDataFolder: () => call<void>('open_data_folder'),
  generateBenchmark: (boardId: string) => call<Patch>('generate_benchmark', { boardId }),
  readyToQuit: () => call<void>('ready_to_quit'),
  readyToClose: () => call<void>('ready_to_close'),

  projectInfo: () => call<ProjectInfo>('project_info'),
  newProject: () => call<boolean>('new_project'),
  openProject: () => call<boolean>('open_project'),
  openLibrary: () => call<void>('open_library'),
  printBoard: () => call<void>('print_board'),
};
