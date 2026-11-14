## What and why

<!-- What does this change do, and why? Link the issue it closes. -->

Closes #

## How it was tested

<!-- Tests added or updated; manual checks in the app (which board, which interaction). -->

- [ ] `npm run lint && npm run typecheck && npm test`
- [ ] `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- [ ] Tried in the running app (`npm run app:dev`)

## Checklist

- [ ] No content is kept only in frontend state; every change is saved through a Rust command.
- [ ] Nothing new blocks the main thread (database and image work stays in Rust, off the UI thread).
- [ ] Documentation updated (`README.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`) if behaviour or architecture changed.
- [ ] New database changes are a new migration file, never an edit to a shipped one.
