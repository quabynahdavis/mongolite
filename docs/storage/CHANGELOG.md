# Storage Documentation Changelog

## [Unreleased]

No changes.

## [0.1.0] – 2026-08-26

Initial storage documentation release.

### Added

- `01-pages.md`:
  - Page header byte layout
  - Page type enum values
  - Integrity checksum verification
- `02-btree.md`:
  - Leaf/internal node structures
  - Insertion/splitting algorithm walkthrough
  - Delete compaction steps
- `03-wal.md`:
  - Companion file naming convention
  - Record header format
  - Replay/flush/checkpoint flow
