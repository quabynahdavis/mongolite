# Documentation Changelog

## v0.1.0 – Initial Documentation Release (2026-08-26)

### Added
- `OVERVIEW.md` – Comprehensive project overview, architecture diagram,
  repository layout, quick start examples in Rust and C.
- `API.md` – Full API reference covering `Database`, `Collection`,
  `Cursor`, query operators, storage engine primitives, and error types.
- Domain-level documentation suites:
  - `architecture/` (`01-file-format.md`, `02-storage-engine.md`,
    `03-document-model.md`)
  - `storage/` (`01-pages.md`, `02-btree.md`, `03-wal.md`)
  - `query/` (`01-query-operators.md`, `02-update-operators.md`,
    `03-projection.md`)
  - `api/` (`01-database.md`, `02-collection.md`, `03-cursor.md`)
- Each domain folder contains `OVERVIEW.md` and `CHANGELOG.md` per
  project mandates.

### Changed
- Replaced previous terse `OVERVIEW.md` with a richer architectural and
  conceptual explanation.
- Added cross-references between storage, query, and API documentation.

### Conventions
- File naming: `<NN-topic>.md` for sequential topics within each domain.
- `v0.1.0` marks the first fully documented baseline for the MongoLite
  codebase.
