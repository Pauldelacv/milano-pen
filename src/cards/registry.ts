// Card kind → component. Adding a card type means adding a component here
// (and, if it has its own data, a type in types/models.ts).
import type { ComponentType } from 'react';
import type { Card, CardKind } from '../types/models';
import { BoardCard } from './BoardCard';
import { FileCard } from './FileCard';
import { ImageCard } from './ImageCard';
import { LinkCard } from './LinkCard';
import { NoteCard } from './NoteCard';
import { SwatchCard } from './SwatchCard';
import { UnsupportedCard } from './UnsupportedCard';

export interface CardProps {
  card: Card;
  editing: boolean;
  selected: boolean;
}

const REGISTRY: Partial<Record<CardKind, ComponentType<CardProps>>> = {
  note: NoteCard,
  image: ImageCard,
  file: FileCard,
  link: LinkCard,
  board: BoardCard,
  swatch: SwatchCard,
};

export function cardComponent(kind: CardKind): ComponentType<CardProps> {
  return REGISTRY[kind] ?? UnsupportedCard;
}

export const KIND_LABELS: Record<CardKind, string> = {
  note: 'Note',
  document: 'Document',
  todo: 'To-do list',
  image: 'Image',
  file: 'File',
  link: 'Link',
  column: 'Column',
  board: 'Board',
  table: 'Table',
  comment: 'Comment',
  swatch: 'Colour swatch',
  sketch: 'Sketch',
  line: 'Line',
  shortcut: 'Board shortcut',
};
