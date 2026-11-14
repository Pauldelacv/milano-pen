// Font menu for notes: macOS fonts, bundled Google Fonts, or any installed
// font typed by name.
import { useState } from 'react';
import { saveQueue } from '../app/actions';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';
import { customFamily, customFontId, FONT_CHOICES, fontStack } from './fonts';

const OTHER = '__other__';

export function setNoteFont(ids: string[], font: string | null) {
  const store = useBoardStore.getState();
  store.setLocal(ids.map((id) => ({ id, data: { font } })));
  for (const id of ids) saveQueue.schedule({ id, data: { font } }, 'Change font', `font:${ids.join(',')}`);
  void saveQueue.flush();
}

export function FontPicker({ cardIds, compact = false, onDone }: { cardIds: string[]; compact?: boolean; onDone?: () => void }) {
  const current = useBoardStore((s) => s.cards[cardIds[0]]?.data.font as string | undefined);
  const [typing, setTyping] = useState(false);
  const [name, setName] = useState(customFamily(current) ?? '');
  const custom = customFamily(current);

  if (typing) {
    return (
      <form
        className="font-form"
        onSubmit={(e) => {
          e.preventDefault();
          const id = customFontId(name);
          if (!id) {
            useUiStore.getState().toast('Type the name of a font installed on your Mac, e.g. “Source Sans 3”.', { kind: 'error' });
            return;
          }
          setNoteFont(cardIds, id);
          setTyping(false);
          onDone?.();
        }}
      >
        <input
          autoFocus
          value={name}
          placeholder="Installed font name"
          aria-label="Installed font name"
          spellCheck={false}
          style={{ fontFamily: fontStack(customFontId(name) ?? undefined) }}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.key === 'Escape' && (e.stopPropagation(), setTyping(false))}
        />
        <button type="submit" className="fmt-text">
          Apply
        </button>
      </form>
    );
  }

  return (
    <select
      className={`font-select${compact ? ' compact' : ''}`}
      title="Font"
      aria-label="Font"
      value={custom ? (current as string) : (current ?? 'system')}
      onChange={(e) => {
        const v = e.target.value;
        if (v === OTHER) return setTyping(true);
        setNoteFont(cardIds, v === 'system' ? null : v);
        onDone?.();
      }}
    >
      {(['System', 'Google Fonts'] as const).map((group) => (
        <optgroup key={group} label={group === 'System' ? 'macOS' : 'Google Fonts (bundled)'}>
          {FONT_CHOICES.filter((f) => f.group === group).map((f) => (
            <option key={f.id} value={f.id} style={{ fontFamily: f.stack }}>
              {f.label}
            </option>
          ))}
        </optgroup>
      ))}
      <optgroup label="Installed on this Mac">
        {custom && <option value={current}>{custom}</option>}
        <option value={OTHER}>Other font…</option>
      </optgroup>
    </select>
  );
}
