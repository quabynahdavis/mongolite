# API Documentation Changelog

## [Unreleased]

No changes since v0.1.0.

## [0.1.0] – 2026-08-26

First full reference documentation for the public API surface.

### Added

- `01-database.md`:
  - `Database::create`, `Database::open`
  - `Database::collection`, `Database::list_collections`
  - `Database::drop_collection`, `Database::flush`
- `02-collection.md`:
  - Methods: `insert_one`, `insert_many`, `find`, `find_one`,
    `find_with_options`, `count`, `update_one`, `update_many`,
    `delete_one`, `delete_many`, `drop`
  - Result structs documented
- `03-cursor.md`:
  - Iterator trait usage
  - `project`, `sort` helpers
