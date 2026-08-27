# Contributing to MongoLite

Thank you for your interest in contributing!

## Development Setup

```bash
git clone https://github.com/mongolite/mongolite.git
cd mongolite
cargo build --all-features
cargo test --all-features
```

## Project Layout

- `crates/mongolite/` — core library
- `crates/mongolite-cli/` — REPL binary
- `crates/mongolite-ffi/` — C FFI bindings
- `docs/` — all project documentation

## Commit Guidelines

- Write atomic, chronological commits.
- Group only related files in one commit.
- Do not add `Co-authored-by:` or AI metadata tags.
- If hooks block you, use `--no-verify`.

## Documentation

All docs live in `docs/`. Every subdirectory must contain:
- `OVERVIEW.md`
- `CHANGELOG.md`

Topic docs use sequential naming: `01-setup.md`, `02-integration.md`, etc.

## Questions?

Open an issue for bugs, feature requests, or design discussions.
