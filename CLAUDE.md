# CLAUDE.md

Guidance for AI coding assistants working in this repository. Humans: see
CONTRIBUTING.md (same rules).

- Read `docs/ARCHITECTURE.md` before changing data flow, and keep it current.
- Rust core (`crates/milano-core`) has no Tauri dependency; test it with
  `cargo test -p milano-core`. Tauri glue lives in `src-tauri/src`.
- All writes go through `services::history::Recorder` inside
  `Workspace::mutate`, which returns a `Patch` and records undo.
- Frontend: UI → `src/app/actions.ts` → `src/services/ipc.ts` → Rust. Apply
  returned patches with `applyPatch`; never reload a board after an edit.
- Checks: `npm run lint && npm run typecheck && npm test`,
  `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
- Update `docs/ROADMAP.md` when a feature lands; never present a partial
  feature as complete.
