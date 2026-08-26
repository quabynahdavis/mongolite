# Storage Domain Overview

This folder documents the inner workings of the MongoLite storage engine.

## Contents

| File           | Topic                              |
|----------------|------------------------------------|
| `01-pages.md`  | Physical page structure            |
| `02-btree.md`  | B+Tree algorithms & node layout    |
| `03-wal.md`    | Write-ahead log format & lifecycle |

## Audience

- Core engine contributors.
- Security auditors inspecting disk integrity.
- Performance analysts profiling I/O paths.

## Key Concepts

- Fixed-size 4 KiB pages mapped into virtual memory.
- B+Tree indexing for catalog and per-collection document storage.
- WAL ensures atomicity and durability in the absence of locks.

## Cross-References

- [Architecture: Storage Engine](../architecture/02-storage-engine.md)
- [File Format](../architecture/01-file-format.md)
