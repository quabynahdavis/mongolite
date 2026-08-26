# Storage Engine Architecture

## Overview

MongoLite implements a **custom, append-friendly B+Tree storage engine** built
directly on top of OS memory mapping (`memmap2`). It avoids the overhead of
traditional SQL database buffer pools by relying on the host OS page cache for
read caching, while enforcing durability via explicit Write-Ahead Logging
(WAL).

```
 ┌─────────────────────────────────────────────────────────────┐
 │                       Database Handle                       │
 └──────────────────────────────┬──────────────────────────────┘
                                │
               ┌────────────────┴────────────────┐
               ▼                                 ▼
    ┌────────────────────┐            ┌────────────────────┐
    │  Catalog (B-Tree)  │            │ User B-Tree Index  │
    └──────────┬─────────┘            └──────────┬─────────┘
               │                                 │
               └────────────────┬────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │  Page Allocator │
                       └────────┬────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │ File (MMIO)     │
                       └─────────────────┘
```

## Core Abstractions

### 1. `File` (`crates/mongolite/src/storage/file.rs`)

Wraps an `std::fs::File` combined with a read/write memory mapping
(`memmap2::MmapMut`).

- **Single responsibility**: Direct raw slice access (`file.page(id)`) and
  mutations (`file.page_mut(id)`).
- **Auto-expansion**: Calling `grow()` doubles the underlying mapped slice using
  `set_len` + re-mapping.
- **Dirty Page Tracking**: Keeps a `HashSet<u32>` of modified page IDs. Calling
  `flush()` executes an OS-level sync of modified ranges via `mmap.flush()`.

### 2. `Allocator` (`crates/mongolite/src/storage/allocator.rs`)

Manages page lifecycle (alloc / free) within the `File`.

- Maintains an in-memory `free_list_head` initialized from the `FileHeader`.
- If `free_list_head != 0`, `allocate()` pops the top page from the free chain.
- If `free_list_head == 0`, `allocate()` forces `file.grow()` and returns the
  first page of the new expanded space.

### 3. `BTree` (`crates/mongolite/src/storage/btree.rs`)

A generic B+Tree engine that operates on arbitrary byte slices (`Key = Vec<u8>`,
`Value = Vec<u8>`).

- **Fan-out**: Configured via `BTreeConfig { order }`. Order defines the max
  keys per node before a split is triggered. Default maximum order is dynamically
  calculated based on page size.
- **Node Splits**: Splits happen proactively on insert if `num_keys >= order`.
  Splitting divides entries into two half-filled pages and promotes the median key to
  the parent node.
- **Leaf Linking**: All leaf pages maintain a forward pointer (`next_leaf_page`),
  enabling **O(1) sequential page traversals** for fast iteration and range scans.

### 4. `Wal` (`crates/mongolite/src/storage/wal.rs`)

Guarantees crash safety.

- Write operations append page deltas to a companion `.mongolite-wal` file.
- Before dirty pages in the main `.mongolite` file are flushed to disk, the WAL is
  synced.
- On startup (`Database::open`), MongoLite checks for an existing WAL file and
  replays uncommitted changes to restore consistency.

### 5. `PagePool` (`crates/mongolite/src/storage/pool.rs`)

An optional LRU cache wrapper around physical pages to minimize re-reading
frequently accessed pages when memory mapping is un-featured (e.g., WASM targets).
