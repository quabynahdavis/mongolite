# Storage Engine

The storage engine is responsible for moving data between disk and memory. It manages page allocation, caching, and the read/write paths that all higher-level components depend on.

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                  Database Layer                       │
├──────────────────────────────────────────────────────┤
│               Collection Layer                        │
├──────────────────────────────────────────────────────┤
│                Query Engine                           │
├──────────────────────────────────────────────────────┤
│                Storage Engine                         │
│  ┌──────────┐  ┌──────────┐  ┌───────────────────┐  │
│  │  Page    │  │  B+tree  │  │  WAL Manager      │  │
│  │  Cache   │  │  Manager │  │  (Write-Ahead Log)│  │
│  └──────────┘  └──────────┘  └───────────────────┘  │
├──────────────────────────────────────────────────────┤
│           Memory-Mapped I/O Layer                     │
├──────────────────────────────────────────────────────┤
│              .mongolite File                          │
└──────────────────────────────────────────────────────┘
```

## Memory-Mapped I/O

MongoLite uses memory-mapped I/O (`mmap`) as its primary mechanism for file access. The entire `.mongolite` file is mapped into the process's virtual address space, allowing the OS to handle paging data in and out of memory.

### Advantages

- **Zero-copy reads** — Data is accessed directly from the OS page cache without copying into user-space buffers.
- **OS-managed caching** — The kernel's page cache policy automatically keeps hot data in memory.
- **Simplified I/O** — No need for explicit `read()`/`write()` syscalls; memory access is sufficient.

### Trade-offs

- **Address space** — Very large databases may require 64-bit address space.
- **Control** — Less fine-grained control over eviction compared to a custom buffer pool.

## Page Cache

While memory-mapped I/O handles the bulk of data access, MongoLite maintains a lightweight **page cache** layer that tracks:

- **Dirty pages** — Pages modified in memory but not yet flushed to disk.
- **Pin count** — Prevents the OS from evicting pages that are actively being used.
- **Page table** — Maps page IDs to their memory addresses.

## Read Path

1. The query engine requests a page by its `page_id`.
2. The storage engine checks the page cache for a pinned reference.
3. If not cached, the page is accessed via the memory-mapped region.
4. The OS loads the page from disk if it is not already in the page cache.
5. The page data is returned to the caller.

## Write Path

1. The query engine requests a page modification.
2. The storage engine first writes the change to the **WAL** (see [WAL documentation](../storage/03-wal.md)).
3. The WAL entry is `fsync`'d to guarantee durability.
4. The page is modified in the memory-mapped region.
5. The page is marked dirty in the page cache.
6. Dirty pages are periodically flushed to disk by the background writer.

## Page Allocation

When a new page is needed:

1. Check the **free list** (linked list of deallocated pages).
2. If the free list is non-empty, pop a page from the head.
3. If the free list is empty, extend the file and allocate from the new region.
4. Update the file header's `page_count` and (if applicable) `free_list_head`.

## Concurrency

The storage engine uses the following concurrency model:

- **Read operations** — Multiple concurrent readers are allowed; no locking required for read-only access.
- **Write operations** — A single writer mutex serializes modifications to prevent corruption.
- **WAL writes** — Appends to the WAL are serialized through a dedicated lock.

> **Note:** Future versions may implement multi-readers-single-writer (MRSW) page-level locking for higher concurrency.

## Background Writer

A background thread periodically flushes dirty pages to disk:

- **Checkpoint interval** — Every 60 seconds (configurable).
- **Dirty threshold** — If more than 25% of pages are dirty, trigger an early checkpoint.
- **WAL truncation** — After a successful checkpoint, old WAL entries are truncated to reclaim space.
