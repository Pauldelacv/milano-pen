import { useCallback, useEffect, useState } from 'react';
import { applyPatch, refreshHistory, run } from '../app/actions';
import { KIND_LABELS } from '../cards/registry';
import { api } from '../services/ipc';
import { useUiStore } from '../stores/uiStore';
import type { TrashEntry } from '../types/models';
import { Modal } from './Modal';

export function TrashDialog() {
  const close = () => useUiStore.getState().setDialog(null);
  const [items, setItems] = useState<TrashEntry[] | null>(null);
  const [confirming, setConfirming] = useState(false);
  const load = useCallback(async () => setItems((await run(api.listTrash())) ?? []), []);
  useEffect(() => {
    void load();
  }, [load]);

  const restore = async (id: string) => {
    const patch = await run(api.restoreTrashed(id));
    if (patch) {
      applyPatch(patch);
      useUiStore.getState().toast('Restored');
      void load();
    }
  };
  const empty = async () => {
    const r = await run(api.emptyTrash());
    setConfirming(false);
    if (r) {
      useUiStore.getState().toast(`Deleted ${r.cards} card${r.cards === 1 ? '' : 's'} permanently`);
      void refreshHistory();
      void load();
    }
  };

  return (
    <Modal title="Trash" onClose={close} wide>
      {items === null ? (
        <p className="muted">Loading…</p>
      ) : items.length === 0 ? (
        <p className="muted">The trash is empty. Deleted cards appear here and can be restored.</p>
      ) : (
        <ul className="trash-list">
          {items.map((t) => (
            <li key={t.id}>
              <div className="trash-main">
                <span className="trash-title">{t.title || KIND_LABELS[t.kind]}</span>
                <span className="muted">
                  {KIND_LABELS[t.kind]} · from {t.boardTitle || 'a deleted board'} · {new Date(t.trashedAt).toLocaleString()}
                </span>
              </div>
              <button className="button" disabled={!t.restorable} title={t.restorable ? '' : 'Restore its board first'} onClick={() => void restore(t.id)}>
                Restore
              </button>
            </li>
          ))}
        </ul>
      )}
      {!!items?.length && (
        <footer className="modal-footer">
          {confirming ? (
            <>
              <span>Permanently delete everything in the trash? This can’t be undone.</span>
              <button className="button" onClick={() => setConfirming(false)}>
                Cancel
              </button>
              <button className="button danger" onClick={() => void empty()}>
                Delete permanently
              </button>
            </>
          ) : (
            <button className="button danger" onClick={() => setConfirming(true)}>
              Empty trash…
            </button>
          )}
        </footer>
      )}
    </Modal>
  );
}
