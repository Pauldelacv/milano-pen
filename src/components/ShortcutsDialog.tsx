import { useUiStore } from '../stores/uiStore';
import { Modal } from './Modal';

const SHORTCUTS: [string, string][] = [
  ['Double-click canvas', 'Add a note'],
  ['⌘N / ⇧⌘N', 'New note / new board'],
  ['⇧⌘I', 'Add images'],
  ['Double-click card / ↩', 'Edit note · open board, image, file or link'],
  ['Esc', 'Stop editing · clear selection'],
  ['⌘Z / ⇧⌘Z', 'Undo / redo'],
  ['⌘C / ⌘X / ⌘V', 'Copy / cut / paste cards, images, links or text'],
  ['⌘D', 'Duplicate'],
  ['⌫', 'Move to Trash'],
  ['⌘A', 'Select all cards'],
  ['Shift-click / drag on canvas', 'Add to selection / select an area'],
  ['Arrows (⇧ for 10)', 'Nudge selection'],
  ['⇧⌘] / ⇧⌘[', 'Bring to front / send to back'],
  ['⌥ while dragging', 'Ignore the grid'],
  ['Space + drag, middle-drag, scroll', 'Pan'],
  ['Pinch, ⌘ + scroll', 'Zoom'],
  ['⌘= / ⌘− / ⌘0 / ⌘1', 'Zoom in / out / 100% / fit'],
  ['⌘K', 'Search'],
  ['⌘↑ / ⇧⌘H', 'Parent board / Home'],
  ['⌘,', 'Settings'],
];

export function ShortcutsDialog() {
  return (
    <Modal title="Keyboard shortcuts" onClose={() => useUiStore.getState().setDialog(null)} wide>
      <dl className="shortcuts">
        {SHORTCUTS.map(([k, v]) => (
          <div key={k}>
            <dt>
              <kbd>{k}</kbd>
            </dt>
            <dd>{v}</dd>
          </div>
        ))}
      </dl>
    </Modal>
  );
}
