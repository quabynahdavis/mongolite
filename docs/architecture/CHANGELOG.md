# Architecture Documentation Changelog

## [Unreleased]

No changes.

## [0.1.0] – 2026-08-26

Initial architectural documentation set.

### Added

- `01-file-format.md`:
  - File header fields
  - Page layout breakdown
  - Page type enums
- `02-storage-engine.md`:
  - Core abstractions (File, Allocator, BTree, Wal, Pool)
  - Insertion/deletion/search algorithms
- `03-document-model.md`:
  - BSON type mapping
  - ObjectId generation scheme
  - Nested document support status
