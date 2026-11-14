# Roadmap

Status of the milestones from the project brief. `[x]` means implemented and
tested; `[~]` partial (the gap is described); `[ ]` not started. Update this
file in the same pull request as the feature.

## Milestone 0 — Audit and architecture ✅

- [x] Audit of Milaclone and reuse decisions — [ARCHITECTURE.md §1](ARCHITECTURE.md#1-audit-of-milaclone-the-reference-project)
- [x] Rust/Tauri architecture, data model, rendering and image strategy — [ARCHITECTURE.md](ARCHITECTURE.md)

## Milestone 1 — Working macOS application ✅

- [x] Tauri 2 app, Rust core, React + TypeScript frontend
- [x] SQLite persistence (WAL, migrations, indexes, transactions)
- [x] Home board, infinite canvas, pan and zoom
- [x] Create, move and resize notes; autosave; data survives restarts
- [x] Native macOS menu bar (app, File, Edit, View, Window, Help)

## Milestone 2 — Essential cards

- [x] Rich-text notes (bold, italic, underline, strike, headings, lists, links, text sizes, colours)
- [x] Images: dialog, Finder drop, clipboard paste, captions, aspect-locked resize, full-screen viewer with navigation
- [ ] Image cropping
- [x] Files: any type, metadata, open with default app, reveal in Finder
- [ ] Audio/video playback inside file cards (needs range requests in the asset protocol)
- [x] Links: cards, YouTube/Vimeo/Spotify embeds, editable title, open externally
- [ ] Link descriptions and preview thumbnails (must be opt-in: no automatic fetching)
- [ ] To-do lists (checkboxes, nesting, reorder, due dates, tags, stars)
- [ ] Columns (data model and benchmark exist; rendering shows "not supported yet")
- [x] Nested boards: icons, colours, card counts, breadcrumbs, rename
- [ ] Board shortcuts (alias cards)
- [ ] Drag cards into boards and columns
- [x] Clipboard: copy/cut/paste cards (deep, across boards), images, URLs, plain text
- [x] Toolbar drag-to-place

## Milestone 3 — Advanced workspace

- [x] Undo/redo for every operation (row-level history in Rust, coalesced typing)
- [x] Search (⌘K) across boards and cards
- [x] Trash: restore, permanent deletion, undo deletion
- [x] Export: Markdown, JSON; import: JSON; database backup and restore; daily auto-backup
- [ ] Lines and arrows between cards (stored and copied correctly; not drawn yet)
- [ ] Sketches, tables, comments, colour swatches, documents (full-screen editor, word count)
- [ ] Labels and filters
- [ ] Templates
- [ ] Centralised task view across boards
- [ ] Milanote import from official exports (reader → card tree → engine, like Milaclone's importer)
- [ ] FTS5 search index (current search is `LIKE` over a derived `search_text` column)

## Milestone 4 — Performance and polish

- [x] Viewport culling with a spatial index; only visible cards mounted
- [x] Camera outside React, rAF-batched; per-card subscriptions and memoisation
- [x] Thumbnail pipeline: content-addressed originals, background 256/1024 px thumbnails, bounded worker pool, per-asset locks, immutable caching, byte-bounded LRU
- [x] Synthetic benchmark board (1,040 cards, 300 images, columns, nested boards, lines) + automated load test
- [x] Dark mode following macOS, manual override
- [ ] Level-of-detail rendering when zoomed far out (all cards in view are still full DOM cards)
- [ ] Measured frame-time and memory numbers on Intel and Apple Silicon (see PERFORMANCE.md)
- [ ] macOS polish: window state restoration, Dock menu, Quick Look for files

## Milestone 5 — Open-source release

- [x] README, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, issue and PR templates
- [x] CI (frontend, Rust, macOS arm64 + x64 builds), security audit, Dependabot
- [x] Release workflow: DMGs for both architectures, checksums, draft release, optional signing/notarization
- [ ] Apple Developer ID signing and notarization configured (needs the maintainers' Apple account)
- [ ] First stable release
