# Changelog

All notable changes to Milano Pen. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/). Pre-releases carry a suffix
(`-beta.N`, `-rc.N`).

## [Unreleased]

### Added

- **Projects** (#11): File ▸ New Project (⌥⌘N) / Open Project (⌘O) create or
  open a separate workspace — a `.milanopen` folder with its own database,
  files and backups — in its own window. Windows close independently, each
  keeps its own undo history, and cards (with their images and files) can be
  copied from one project and pasted into another. The default workspace is
  now called the Library (File ▸ Open Library).
- **Print** (#8): File ▸ Print (⌘P) prints the current view without the
  interface or the grid dots, scaled to a landscape page; use “Save as PDF”
  in the macOS print dialog to get a PDF.
- **Colour swatches** (#10): the Colour tool adds a swatch from a hex code,
  `rgb()`, `hsl()` or a colour name; click the code on the card to copy it.
- **Note fonts** (#9): pick a font in the note editor or the context menu —
  macOS fonts, five bundled Google Fonts (Inter, Lora, Playfair Display,
  Caveat, JetBrains Mono; nothing is downloaded) or any font installed on
  the Mac.
- **Image adjustments** (#12): selecting an image shows a toolbar with
  brightness, contrast, saturation, hue, black & white and sepia sliders.
  Adjustments are non-destructive and undoable.

### Fixed

- Drag and drop from a browser did nothing (#7). Images, links and text can
  now be dropped from browsers and other apps, as well as files from Finder.

## [0.1.0-beta.1] — 2026-11-14

First public beta. Expect rough edges, and back up your data
(**File → Back Up Database…**) before upgrading to a later build.

### Included

- Native macOS app (Tauri 2, Rust core, React + TypeScript), Apple Silicon and
  Intel DMGs, native menu bar and file dialogs.
- Local SQLite storage (WAL, migrations) with autosave; no account, no
  telemetry, no network calls of its own.
- Infinite canvas with pan, zoom and nested boards (icons, colours, card
  counts, breadcrumbs, rename).
- Cards: rich-text notes, images (dialog, Finder drop, paste, captions,
  full-screen viewer), files (open, reveal in Finder), links with
  YouTube/Vimeo/Spotify embeds.
- Copy/cut/paste of cards across boards, toolbar drag-to-place.
- Undo/redo for every operation, search (⌘K), trash with restore.
- Export to Markdown and JSON, JSON import, database backup and restore,
  automatic daily backups.
- Light and dark mode following macOS, with manual override.
- Viewport culling, background thumbnail pipeline and a 1,040-card synthetic
  benchmark board.

### Known limitations

- Builds are **not signed with a Developer ID and not notarized**: Gatekeeper
  blocks the first launch (see the README for how to open the app).
- Not implemented yet: image cropping, audio/video playback in file cards,
  link previews, to-do lists, columns, board shortcuts, dragging cards into
  boards, lines and arrows, sketches, tables, comments, labels, templates,
  task view, Milanote import. Existing cards of unsupported kinds are kept and
  shown as "not supported yet".
- No level-of-detail rendering when zoomed far out; frame-time and memory
  figures have not been measured on real hardware yet
  ([PERFORMANCE.md](docs/PERFORMANCE.md)).
- No automatic updates: download new builds from the Releases page.

[0.1.0-beta.1]: https://github.com/pauldelacv/milano-pen/releases/tag/v0.1.0-beta.1
