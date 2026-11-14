# Milano Pen — Architecture

This document records the technical decisions behind Milano Pen. Update it in
the same pull request as any change that alters them.

## 1. Audit of Milaclone (the reference project)

[Milaclone](https://github.com/ellshep/milaclone) (MIT, © 2026 ellshep) is a
self-hosted Node.js/Express/SQLite web app with a vanilla-JS frontend. What we
learned and what we do with it:

| Milaclone concept | Verdict | In Milano Pen |
| --- | --- | --- |
| `canvases` (tree via `parentCanvasId`) + `items` (`canvasId`, `parentItemId`, `type`, `x/y/w/h/z`, `color`, JSON `data`) | **Reused** — the model is sound and flexible | `boards` + `cards` tables, same shape, snake_case, `REAL` coordinates, `updated_at`, `search_text` |
| A `board` item owns a child canvas; title/colour/icon live on the canvas row | **Reused** | `card.kind = 'board'`, `data.childBoardId`; title/colour/icon on `boards` |
| Soft delete via `trashedBy = <root id>` tagging the whole subtree | **Reused** | `trashed_by` / `trashed_at` on both tables; one Trash entry per delete |
| Deep copy with id remapping (`collectSubtree`, `copyItemDeep`) | **Reused, rewritten in Rust** | `services::cards` (clipboard snapshot, paste, duplicate) |
| Rich text as TipTap JSON (`data.doc`) + plain text (`data.text`) | **Reused** | Same contract; Rust derives `search_text` for every kind |
| Markdown export shape (round-trips with its importer) | **Reused** | `services::export` |
| Embed URL mapping (YouTube/Vimeo/Spotify, built from the URL only, never fetched) | **Reused, ported to TS** | `src/cards/embeds.ts` |
| Label colours hashed from the label name (no registry) | **Reused** (Milestone 3) | |
| Three-way to-do merge (`mergeTasks`) | **Not needed** — single user, single window | |
| Express routes, multer uploads, upload header hardening | **Dropped** — no HTTP server | Tauri IPC + a read-only custom protocol for assets |
| Service worker, PWA, Tailscale, Docker | **Dropped** — native app | |
| Undo limited to deletes | **Replaced** | Row-snapshot history in Rust covering every mutation |
| `render()` clears and re-renders the whole board | **Replaced** | Viewport virtualization + per-card subscriptions |
| Uploads served at full resolution | **Replaced** | Content-addressed asset store + thumbnail pipeline |
| Camera in `localStorage` | **Kept** (UI state only, not content) | |

Where code is ported closely (embed mapping, Markdown export layout, the
board/canvas duality), the Milaclone copyright notice is preserved in
[`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md).

## 2. Process layout

```
┌──────────────────────────── macOS .app ─────────────────────────────┐
│  WKWebView (React + TypeScript)        Rust (Tauri 2)                 │
│  ┌──────────────────────────┐  IPC   ┌─────────────────────────────┐ │
│  │ stores (zustand)          │◄──────►│ commands/  (thin, async)    │ │
│  │ canvas/ (camera, culling) │ invoke │      │                      │ │
│  │ cards/, editors/ (Tiptap) │ events │ milano-core (no Tauri deps) │ │
│  └──────────┬───────────────┘        │  database/ models/ services/│ │
│             │ <img src="milano://…">  │  files/ images/             │ │
│             └────────────────────────►│ asset protocol (read-only)  │ │
│                                       └─────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
          Application Support/Milano Pen/{database,assets,thumbnails,cache,backups}
```

### Repository layout (deviation from the suggested one, and why)

The suggested `src-tauri/src/{database,models,services,files,images}` modules
live in a separate crate, **`crates/milano-core`**, with no Tauri dependency.
Reason: the whole data layer can be unit-tested with `cargo test` on any OS in
seconds, without a webview toolchain, and could later back a CLI importer or
another shell. `src-tauri` only contains IPC commands, menus, window events
and the asset protocol. Migrations live in `crates/milano-core/migrations/`.

```
crates/milano-core/src/
  database/   connection, pragmas, migration runner
  models/     Board, Card, Asset, Patch (IPC-facing, camelCase JSON)
  services/   boards, cards, history (undo/redo), trash, search, export, backup, benchmark
  files/      content-addressed asset store
  images/     dimensions, EXIF orientation, thumbnails, byte-bounded LRU
src-tauri/src/
  commands/   one file per domain, all `async`
  state.rs    open projects, window → project map, shared clipboard
  windows.rs  project windows, close/quit flush handshakes
  menu.rs     native macOS menu bar
  protocol.rs milano:// asset protocol
src/          React app (app/, canvas/, cards/, editors/, stores/, services/, styles/, types/)
```

## 3. Data model

SQLite (`rusqlite`, bundled SQLite), `PRAGMA journal_mode=WAL`,
`synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout`. Schema version is
`PRAGMA user_version`; migrations are embedded SQL files applied in order, each
in its own transaction, after a `VACUUM INTO` safety copy of any existing
database.

```
meta    (key PK, value)                       root_board_id, settings
boards  (id PK, parent_board_id, title, color, icon,
         created_at, updated_at, trashed_by, trashed_at)
cards   (id PK, board_id, parent_card_id, kind, x, y, w, h, z, color,
         data JSON, search_text, created_at, updated_at, trashed_by, trashed_at)
assets  (id PK = sha256, ext, mime, size, width, height, original_name, created_at)
```

* IDs are UUID v7 (time-ordered → good B-tree locality).
* `cards.data` is per-kind JSON (validated at the edges, typed in
  `src/types/models.ts`); `search_text` is derived by Rust on every write so
  search needs no per-kind SQL.
* Partial indexes cover the hot paths: live cards of a board, column children,
  trash roots, asset references.
* Every read filters `trashed_by IS NULL`.

## 4. Mutations, autosave and undo/redo

All writes go through one path: `Recorder::put_card` / `put_board`, inside a
transaction. The recorder snapshots each touched row **before** its first write
and **after** the transaction. That pair list is:

1. returned to the frontend as a `Patch` (`cards`, `removedCards`, `boards`) so
   the UI updates incrementally — the board is never reloaded after an edit;
2. pushed on the undo stack (bounded, in memory, per session).

Undo writes the "before" rows back, redo the "after" rows; both return a Patch.
Because history is row-level, every operation (create, move, resize, edit, z-order,
trash, restore, paste, duplicate, board rename) is undoable without
per-command inverse code. Consecutive edits with the same coalescing key
(e.g. typing in one note) within 1.5 s merge into one undo step.

Autosave: pointer interactions mutate only the frontend store; the final
geometry is persisted in one batched command on `pointerup`. Text edits are
debounced (400 ms) and flushed on blur, board switch, window blur and app quit
(Rust intercepts the quit request, asks the webview to flush, then exits).

## 5. Rendering strategy

* DOM cards (rich text and inputs need the DOM), positioned in world
  coordinates inside one `world` layer transformed with
  `translate3d(x,y,0) scale(s)`.
* The camera lives outside React. Wheel/pinch/drag update it and a single
  `requestAnimationFrame` applies the transform and recomputes the visible set.
* **Spatial index:** uniform grid hash (512-unit cells) over card bounds,
  updated incrementally. The viewport + overscan query returns candidate ids;
  React state changes only when the visible id set changes.
* Only visible cards are mounted. Each card subscribes to its own record
  (`useBoardStore(s => s.cards[id])`) and is `memo`ised, so moving one card
  re-renders one card.
* Non-edited notes render static HTML generated from the Tiptap JSON (cached
  per document); a Tiptap editor is mounted only for the card being edited.
* Lines/drawings (Milestone 3) will use a separate SVG/canvas layer.

## 6. Asset & image pipeline

1. Import (dialog, drag and drop, clipboard): Rust streams the file into
   `assets/<aa>/<sha256>.<ext>` (content-addressed → deduplicated, originals
   never modified) and reads only the header for dimensions + EXIF orientation.
   The card is created immediately with the aspect ratio known.
2. Thumbnails (256 px and 1024 px, JPEG q82 or lossless WebP when the image has
   alpha) are generated in the background on a bounded worker pool
   (`tokio::Semaphore`, ≤ 4 concurrent decodes), each source decoded once.
3. The webview loads `milano://localhost/asset/<sha256>/<variant>`. The
   protocol only serves files from the asset and thumbnail directories, by
   hash; a missing thumbnail is generated on demand (same pool, same per-asset
   lock, so no duplicate work). Responses are `immutable` — the URL is the
   content hash, which is the cache-invalidation mechanism.
4. The frontend picks the variant from on-screen size × devicePixelRatio, and
   requests the original only when zoomed in past the 1024 variant or in the
   viewer. `<img decoding="async" loading="lazy">` and a placeholder box with
   the right aspect ratio avoid layout shifts.
5. A byte-bounded in-memory LRU (64 MB) caches hot thumbnails in Rust; the
   browser's own cache is bounded by unmounting off-screen cards.

## 7. Security & privacy

* No network access is made by the app. No telemetry, no accounts.
* Tauri capabilities grant the webview only core events/window close and the
  app's own commands. No `fs` plugin, no shell. External links open through a
  Rust command that accepts only `http(s)`/`mailto` URLs.
* File paths are never trusted from the frontend: import commands accept only
  paths the user picked in a native dialog. Drag and drop is handled by the
  webview (Tauri's native drop handler is disabled, because it swallowed
  browser drags): dropped and pasted files reach Rust as bytes. Images a
  browser offers only as a URL become link cards — nothing is downloaded.
* CSP restricts scripts to the bundle, images to `self`/`milano:`/`data:`/`blob:`,
  frames to the YouTube-nocookie/Vimeo/Spotify embed hosts. Link previews are
  never fetched automatically.

## 8. Windows and projects

* A **project** is a workspace folder — `Name.milanopen/` with the layout of
  §10 (database, assets, thumbnails, backups) — so it can be moved, copied or
  backed up as one item. The **Library** is the default workspace in
  Application Support, shown by the `main` window; it also stores the app
  settings (theme, snapping…), which apply to every window.
* Each window shows one project. `AppState` maps window label → `Project`
  (its `Workspace`, thumbnailer and paths); every command resolves the
  project from the calling webview, and the `milano://` protocol from the
  requesting webview's label, so a window can only reach its own data. A
  project open in one window is focused rather than opened twice.
* Undo history is per project (it lives in its `Workspace`).
* The clipboard is app-wide: copying stores a snapshot plus the source
  project; pasting into another project first copies the referenced asset
  originals and rows (`Workspace::snapshot_assets` / `adopt_assets`), then
  pastes the snapshot.
* Events from Rust are sent to one window (`emit_to`), and the webview
  listens on its own window only, so menus, flush requests and restores never
  leak between windows. Closing a window asks its webview to flush (with a
  timeout) before it closes; quitting does the same for every window.

## 9. Printing

⌘P calls `Webview::print` (the macOS print dialog, which also saves PDF).
A print stylesheet hides the interface, selection and grid dots and scales
the current view to fit a landscape page; the frontend computes the scale.

## 10. Data location

`~/Library/Application Support/Milano Pen/` (resolved with Tauri's
`app_data_dir` for the `productName`; on other OSes the platform equivalent):

```
database/milano-pen.sqlite   (+ -wal, -shm)
assets/       originals, content-addressed
thumbnails/   generated, disposable
cache/        disposable
backups/      automatic daily VACUUM INTO snapshots (last 10 kept) + pre-migration copies
```

Projects use the same layout inside their `.milanopen` folder.

## 11. Future-proofing (not implemented)

* **Sync/collaboration:** row-level change sets with UUIDs and `updated_at`
  are the natural unit for a future operation log / CRDT bridge.
* **Search:** `search_text` can feed an FTS5 table without schema churn.
* **Milanote import:** an importer will map an official export to the same
  card tree Milaclone's importer uses (readers → one engine). No private APIs.
* **Linux/Windows:** nothing macOS-specific outside `menu.rs` and bundle config.
