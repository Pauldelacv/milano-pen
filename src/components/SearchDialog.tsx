import { LayoutGrid, StickyNote } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { centerOnCard, openBoard, run } from '../app/actions';
import { KIND_LABELS } from '../cards/registry';
import { api } from '../services/ipc';
import { useBoardStore } from '../stores/boardStore';
import { useUiStore } from '../stores/uiStore';
import type { SearchResult } from '../types/models';
import { Modal } from './Modal';

export function SearchDialog() {
  const close = () => useUiStore.getState().setDialog(null);
  const [q, setQ] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [active, setActive] = useState(0);
  const seq = useRef(0);

  useEffect(() => {
    const n = ++seq.current;
    const t = setTimeout(async () => {
      const r = q.trim() ? await run(api.search(q)) : [];
      if (n === seq.current) {
        setResults(r ?? []);
        setActive(0);
      }
    }, 120);
    return () => clearTimeout(t);
  }, [q]);

  const choose = async (r: SearchResult) => {
    close();
    if (useBoardStore.getState().boardId !== r.boardId && !(await openBoard(r.boardId))) return;
    if (r.kind === 'card') {
      useUiStore.getState().select([r.id]);
      requestAnimationFrame(() => centerOnCard(r.id));
    }
  };

  return (
    <Modal title="Search" onClose={close} wide>
      <input
        className="search-input"
        autoFocus
        placeholder="Search boards and cards"
        value={q}
        onChange={(e) => setQ(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'ArrowDown') {
            e.preventDefault();
            setActive((a) => Math.min(a + 1, results.length - 1));
          }
          if (e.key === 'ArrowUp') {
            e.preventDefault();
            setActive((a) => Math.max(a - 1, 0));
          }
          if (e.key === 'Enter' && results[active]) {
            e.preventDefault();
            e.stopPropagation();
            void choose(results[active]);
          }
        }}
      />
      <ul className="result-list" role="listbox">
        {results.map((r, i) => (
          <li key={`${r.kind}:${r.id}`} role="option" aria-selected={i === active}>
            <button className={`result${i === active ? ' active' : ''}`} onMouseEnter={() => setActive(i)} onClick={() => void choose(r)}>
              {r.kind === 'board' ? <LayoutGrid size={16} /> : <StickyNote size={16} />}
              <span className="result-main">
                <span className="result-title">{r.title}</span>
                <span className="result-path">
                  {r.kind === 'card' && r.cardKind ? `${KIND_LABELS[r.cardKind]} · ` : 'Board · '}
                  {r.path.join(' › ')}
                </span>
              </span>
            </button>
          </li>
        ))}
        {q.trim() && !results.length && <li className="muted">No matches</li>}
      </ul>
    </Modal>
  );
}
