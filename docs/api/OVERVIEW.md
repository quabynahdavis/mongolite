# API Domain Overview

This folder documents the public Rust APIs exposed by the `mongolite` crate.

## Contents

| File                 | Purpose                               |
|----------------------|----------------------------------------|
| `01-database.md`     | Entry point for file creation/opening  |
| `02-collection.md`   | CRUD operations                        |
| `03-cursor.md`       | Lazy iteration and helpers             |

## Conventions

- Code samples assume `use mongolite::Database;` and the latest stable Rust.
- BSON documents are created inline via `bson::doc! { … }`.
- Errors propagate through `Result<T, Error>`.

## External References

- [Storage Engine Docs](../storage/02-btree.md)
- [Query Language Docs](../query/01-query-operators.md)
