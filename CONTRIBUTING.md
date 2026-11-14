# Contributing to Milano Pen

Thanks for helping! Milano Pen aims to be a dependable, fast, local-first
visual workspace. This guide covers how to set up, what we expect from a
change, and where things live.

## Getting started

1. Install the prerequisites listed in the [README](README.md#development).
2. Fork and clone the repository, then:
   ```bash
   npm install
   MILANO_PEN_DATA_DIR="$PWD/.milano-dev-data" npm run app:dev
   ```
3. Pick something from [docs/ROADMAP.md](docs/ROADMAP.md) or an open issue.
   For anything larger than a small fix, open an issue first so we can agree
   on the approach.

## Ground rules

- **No fake UI.** Buttons and menu items must work. If a feature is partial,
  say so in the UI and in `docs/ROADMAP.md` rather than presenting a placeholder.
- **Content is saved by Rust.** The frontend may update its store optimistically,
  but every change must reach a Rust command. Never keep important content only
  in React state or `localStorage` (which holds view state only, like the camera).
- **Don't block the UI thread.** Database and image work runs in Rust commands,
  which are `async` and use `AppState::with` (a blocking pool). Pointer moves
  update local state; persistence happens once at the end of an interaction.
- **Don't reload the board after an edit.** Commands return a `Patch`; apply it.
- **Every mutation goes through `Recorder::put_card`/`put_board`** so it is
  undoable and reported to the UI. Don't write `cards`/`boards` rows directly.
- **Schema changes are new migrations** (`crates/milano-core/migrations/000N_*.sql`
  plus an entry in `database/migrations.rs`). Never edit a shipped migration.
- **Privacy:** no telemetry, no automatic network requests. Anything that
  would contact a server (e.g. link previews) must be explicit and opt-in.
- **Styling uses tokens.** Colours, radii and shadows come from
  `src/styles/tokens.css`; don't hard-code new colours in components.
- **Tests come with the change.** Rust logic gets unit or integration tests in
  `crates/milano-core`; pure frontend logic gets Vitest tests next to the module.

## Adding a card type

1. Pick or add the kind in `CARD_KINDS` (`crates/milano-core/src/models/mod.rs`)
   and `CardKind` (`src/types/models.ts`).
2. Describe its `data` shape in `src/types/models.ts`; make sure
   `services/text.rs` extracts its searchable text.
3. Write the component in `src/cards/` and register it in `src/cards/registry.ts`.
4. Add a toolbar entry or menu command only once it fully works.
5. Cover export (`services/export.rs`) and copy/paste references (`remap_refs`
   in `services/cards.rs`) if the card points at other cards or boards.

## Checks

Run before pushing (CI runs the same):

```bash
npm run lint && npm run typecheck && npm test && npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Commits and pull requests

- Keep pull requests focused; describe what changed and how you tested it
  (the PR template has a checklist).
- Write commit messages in the imperative ("Add column cards"), with a body
  explaining *why* when it isn't obvious.
- Update `docs/ARCHITECTURE.md` when you change how things fit together and
  tick or add items in `docs/ROADMAP.md`.

## Code of Conduct

By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

By contributing, you agree that your contributions are licensed under the
[MIT License](LICENSE).
