// Debounced persistence of card edits (typing, nudges). Each card's changes
// are merged until the debounce fires, then sent in one batch. `flush()`
// sends everything immediately (blur, board switch, quit).
import type { CardUpdate, Patch } from '../types/models';

export interface SaveQueueOptions {
  delayMs: number;
  send: (updates: CardUpdate[], coalesce: string | undefined, label: string) => Promise<Patch>;
  onPatch: (patch: Patch) => void;
  onError: (e: unknown) => void;
  onBusyChange?: (busy: boolean) => void;
}

interface Pending {
  update: CardUpdate;
  coalesce?: string;
  label: string;
}

export class SaveQueue {
  private pending = new Map<string, Pending>();
  private inFlight = new Map<string, number>();
  private timer: ReturnType<typeof setTimeout> | null = null;
  private chain: Promise<void> = Promise.resolve();

  constructor(private readonly opts: SaveQueueOptions) {}

  /** Queue a change for `update.id`; later calls merge into earlier ones. */
  schedule(update: CardUpdate, label = 'Edit', coalesce?: string): void {
    const prev = this.pending.get(update.id);
    const merged: CardUpdate = prev
      ? { ...prev.update, ...update, data: prev.update.data || update.data ? { ...prev.update.data, ...update.data } : undefined }
      : update;
    this.pending.set(update.id, { update: merged, coalesce: coalesce ?? prev?.coalesce, label });
    this.busyChanged();
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => void this.flush(), this.opts.delayMs);
  }

  /** True while `id` has unsaved or unacknowledged changes. */
  isPending(id: string): boolean {
    return this.pending.has(id) || (this.inFlight.get(id) ?? 0) > 0;
  }

  get busy(): boolean {
    return this.pending.size > 0 || this.inFlight.size > 0;
  }

  /** Send everything now; resolves once every queued save has completed. */
  flush(): Promise<void> {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    const batch = [...this.pending.values()];
    this.pending.clear();
    if (batch.length) {
      // One request per coalescing key keeps undo steps per card.
      const groups = new Map<string, Pending[]>();
      for (const p of batch) {
        const key = p.coalesce ?? `label:${p.label}`;
        groups.set(key, [...(groups.get(key) ?? []), p]);
      }
      for (const group of groups.values()) {
        const ids = group.map((p) => p.update.id);
        ids.forEach((id) => this.inFlight.set(id, (this.inFlight.get(id) ?? 0) + 1));
        this.chain = this.chain.then(async () => {
          try {
            const patch = await this.opts.send(
              group.map((p) => p.update),
              group[0].coalesce,
              group[0].label,
            );
            ids.forEach((id) => this.release(id));
            this.opts.onPatch(patch);
          } catch (e) {
            ids.forEach((id) => this.release(id));
            this.opts.onError(e);
          } finally {
            this.busyChanged();
          }
        });
      }
    }
    this.busyChanged();
    return this.chain;
  }

  private release(id: string) {
    const n = (this.inFlight.get(id) ?? 1) - 1;
    if (n <= 0) this.inFlight.delete(id);
    else this.inFlight.set(id, n);
  }

  private lastBusy = false;
  private busyChanged() {
    const b = this.busy;
    if (b !== this.lastBusy) {
      this.lastBusy = b;
      this.opts.onBusyChange?.(b);
    }
  }
}
