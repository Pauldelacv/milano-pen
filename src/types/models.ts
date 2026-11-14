// Mirrors of the Rust IPC types (crates/milano-core/src/models). Keep in step.
import type { JSONContent } from '@tiptap/core';

export type CardKind =
  | 'note'
  | 'document'
  | 'todo'
  | 'image'
  | 'file'
  | 'link'
  | 'column'
  | 'board'
  | 'table'
  | 'comment'
  | 'swatch'
  | 'sketch'
  | 'line'
  | 'shortcut';

export interface Board {
  id: string;
  parentBoardId: string | null;
  title: string;
  color: string | null;
  icon: string | null;
  createdAt: number;
  updatedAt: number;
  cardCount: number;
}

export interface Card<D = CardData> {
  id: string;
  boardId: string;
  parentCardId: string | null;
  kind: CardKind;
  x: number;
  y: number;
  w: number;
  /** Fixed height, or null when the card grows with its content. */
  h: number | null;
  z: number;
  color: string | null;
  data: D;
  createdAt: number;
  updatedAt: number;
}

export interface Crumb {
  id: string;
  title: string;
  color: string | null;
}

export interface BoardView {
  board: Board;
  breadcrumb: Crumb[];
  cards: Card[];
  childBoards: Board[];
}

export interface Patch {
  cards: Card[];
  removedCards: string[];
  boards: Board[];
  removedBoards: string[];
  created: string[];
  label: string;
}

export interface NewCard {
  boardId: string;
  parentCardId?: string | null;
  kind: CardKind;
  x: number;
  y: number;
  w?: number;
  h?: number;
  color?: string | null;
  data?: Record<string, unknown>;
}

export interface CardUpdate {
  id: string;
  x?: number;
  y?: number;
  w?: number;
  h?: number;
  z?: number;
  /** "" clears the colour. */
  color?: string;
  /** Merged shallowly; null removes a key. */
  data?: Record<string, unknown>;
}

export interface BoardUpdate {
  title?: string;
  color?: string;
  icon?: string;
}

export interface SearchResult {
  kind: 'board' | 'card';
  id: string;
  boardId: string;
  cardKind: CardKind | null;
  title: string;
  path: string[];
}

export interface TrashEntry {
  id: string;
  kind: CardKind;
  title: string;
  boardId: string;
  boardTitle: string;
  restorable: boolean;
  trashedAt: number;
}

export interface HistoryState {
  canUndo: boolean;
  canRedo: boolean;
  undoLabel: string | null;
  redoLabel: string | null;
}

export interface Settings {
  theme: 'system' | 'light' | 'dark';
  wheelZooms: boolean;
  snapToGrid: boolean;
  gridSize: number;
  autoBackup: boolean;
}

export interface ImportResult {
  patch: Patch;
  errors: string[];
}

export interface AppInfo {
  version: string;
  dataDir: string;
  platform: string;
}

export interface ProjectInfo {
  name: string;
  path: string;
  /** The default workspace in Application Support (vs a project folder). */
  isLibrary: boolean;
}

export interface IpcError {
  kind: string;
  message: string;
}

// ---- Per-kind card data ------------------------------------------------------

export type TextSize = 'small' | 'normal' | 'large';

export interface NoteData {
  doc?: JSONContent;
  /** Plain text of `doc` (search, exports, fallback rendering). */
  text?: string;
  textSize?: TextSize;
  /** Font id from `editors/fonts.ts`, or "custom:<family>" for an installed font. */
  font?: string;
}

export interface ImageData {
  assetId: string;
  name?: string;
  caption?: string;
  width?: number;
  height?: number;
  mime?: string;
  /** Non-destructive colour adjustments (the original file is never modified). */
  adjust?: ImageAdjust;
}

/** CSS filter amounts. Percentages are 100 = unchanged; hue is in degrees. */
export interface ImageAdjust {
  brightness?: number;
  contrast?: number;
  saturation?: number;
  hue?: number;
  grayscale?: number;
  sepia?: number;
}

export interface SwatchData {
  /** "#RRGGBB". */
  hex: string;
}

export interface FileData {
  assetId: string;
  name?: string;
  mime?: string;
  size?: number;
}

export interface LinkData {
  url: string;
  title?: string;
  description?: string;
}

export interface BoardCardData {
  childBoardId: string;
}

export type CardData = Record<string, unknown>;
