# Query Documentation Changelog

## [Unreleased]

No changes.

## [0.1.0] – 2026-08-26

Initial query language documentation.

### Added

- `01-query-operators.md`:
  - Comparison (`$eq`, `$ne`, `$gt`, ..., `$lte`)
  - Logical (`$and`, `$or`, `$nor`, `$not`)
  - Element (`$exists`, `$type`)
  - Evaluation (`$regex`)
  - Array (`$in`, `$nin`, `$all`, `$size`, `$elemMatch`)
- `02-update-operators.md`:
  - `$set`, `$unset`
  - Filter `_id` requirement note
- `03-projection.md`:
  - Projection rules
  - Sorting behavior
  - Skip/limit semantics
