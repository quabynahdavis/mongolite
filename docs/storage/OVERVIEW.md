# Storage Documentation

This folder covers the internal storage mechanisms of MongoLite. These documents describe how data is physically organized, indexed, and protected against crashes.

## Contents

| File | Description |
|------|-------------|
| [01-pages](./01-pages.md) | Page-based storage: layout, allocation, and the free list |
| [02-btree](./02-btree.md) | B+tree indexes: structure, operations, and concurrency |
| [03-wal](./03-wal.md) | Write-ahead log: crash recovery and durability guarantees |

## Architecture Overview

MongoLite's storage layer is a page-oriented system:

```
File
└── Pages (fixed-size blocks, typically 4 KiB)
    ├── Header page      — file metadata
    ├── Index pages      — B+tree nodes for fast lookups
    ├── Data pages       — raw document storage
    ├── Catalog pages    — collection metadata
    └── Overflow pages   — continuation for large documents
```

The **B+tree** layer provides efficient index lookups over these pages. The **WAL** layer ensures that modifications are durable even if the process crashes mid-write.

## Relationship to Other Components

```
Query Engine
     │
     ▼
Storage Engine (see architecture docs)
     │
     ├── Page Manager  ←  docs/storage/01-pages.md
     ├── B+tree        ←  docs/storage/02-btree.md
     └── WAL           ←  docs/storage/03-wal.md
```

## Reading Order

For a complete understanding of MongoLite's storage internals:

1. Start with [01-pages.md](./01-pages.md) to understand the basic unit of storage.
2. Continue with [02-btree.md](./02-btree.md) to learn how indexes are structured.
3. Finish with [03-wal.md](./03-wal.md) to understand durability and crash recovery.
