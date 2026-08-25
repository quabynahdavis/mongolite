# Pages

Pages are the fundamental unit of storage in MongoLite. All data — documents, indexes, and metadata — is organized into fixed-size pages within the `.mongolite` file.

## Page Size

The default page size is **4096 bytes** (4 KiB). This can be configured at database creation time:

| Size | Use Case |
|------|----------|
| 4096 | Default; good for most workloads |
| 8192 | Large documents, fewer page splits |
| 16384 | Analytical workloads, sequential scans |
| 32768 | Very large documents, bulk inserts |

Once set, the page size cannot be changed without recreating the database.

## Page Layout

Every page has a small header followed by type-specific data:

```
┌─────────────────────────────────────────┐
│ Page Header (12 bytes)                  │
│   - page_type  (2 bytes)                │
│   - flags      (2 bytes)                │
│   - page_id    (4 bytes)                │
│   - checksum   (4 bytes)                │
├─────────────────────────────────────────┤
│ Page Data (page_size - 12 bytes)        │
│   - type-specific content               │
└─────────────────────────────────────────┘
```

## Page Types

### Header Page (Page 0)

The first page of the file. Contains the file header with global metadata (see [File Format](../architecture/01-file-format.md)).

### Index Leaf Page

Stores the leaf nodes of a B+tree index. Contains sorted key-pointer pairs:

```
┌─────────────────────────────────────────┐
│ Page Header                             │
├─────────────────────────────────────────┤
│ Number of entries (4 bytes)             │
│ Right sibling page ID (4 bytes)         │
├─────────────────────────────────────────┤
│ Key 1 │ Value 1 (document pointer)      │
│ Key 2 │ Value 2                         │
│ ...                                    │
│ Key N │ Value N                         │
└─────────────────────────────────────────┘
```

### Index Internal Page

Stores the internal (non-leaf) nodes of a B+tree. Contains keys and child page pointers:

```
┌─────────────────────────────────────────┐
│ Page Header                             │
├─────────────────────────────────────────┤
│ Number of children (4 bytes)            │
├─────────────────────────────────────────┤
│ Child 0 │ Key 1 │ Child 1 │ Key 2 │ ...│
│ ...        │ Child N                      │
└─────────────────────────────────────────┘
```

### Data Page

Stores raw document data. Documents are packed sequentially:

```
┌─────────────────────────────────────────┐
│ Page Header                             │
├─────────────────────────────────────────┤
│ Free space offset (2 bytes)             │
│ Number of records (2 bytes)             │
├─────────────────────────────────────────┤
│ Slot Array (grows from start)           │
│   [offset: 2, length: 2]               │
│   [offset: 2, length: 2]               │
│   ...                                  │
├─────────────────────────────────────────┤
│ Free Space                              │
├─────────────────────────────────────────┤
│ Records (grow from end)                 │
│   ┌───────────────────────┐            │
│   │ BSON Document         │            │
│   └───────────────────────┘            │
│   ┌───────────────────────┐            │
│   │ BSON Document         │            │
│   └───────────────────────┘            │
└─────────────────────────────────────────┘
```

### Catalog Page

Stores collection metadata: names, index definitions, and statistics.

### Overflow Page

When a document exceeds the available space in a data page, it spills into one or more overflow pages linked together.

### Free Page

A page that has been deallocated and is available for reuse. Free pages form a singly-linked list.

## Page Allocation

### Allocation Strategy

1. **Free list first** — Check the free list for a reusable page.
2. **Extend file** — If the free list is empty, allocate from the end of the file.

### File Extension

When the file needs to grow:

| Current Size | Extension Amount |
|--------------|------------------|
| < 1 MiB | Double the file |
| 1 MiB – 64 MiB | Double the file |
| > 64 MiB | Add 64 MiB |

This doubling strategy balances between frequent small allocations and excessive memory reservation.

## Free List

The free list is a singly-linked list of deallocated pages. The head pointer is stored in the file header.

```
File Header
    │
    ▼
┌────────┐    ┌────────┐    ┌────────┐
│ Page 4 │───▶│ Page 9 │───▶│ Page 2 │───▶ 0 (null)
│ (free) │    │ (free) │    │ (free) │
└────────┘    └────────┘    └────────┘
```

When a page is freed:
1. Its `page_type` is set to `PAGE_FREE`.
2. The next free page ID is written into the page's data area.
3. The file header's `free_list_head` is updated to point to this page.

## Page Pinning

The page cache tracks **pin counts** to prevent the OS from evicting pages that are actively in use:

- A page is **pinned** when a component holds a reference to it.
- A page is **unpinned** when the reference is released.
- Only **unpinned** pages can be evicted from the cache.

```rust
// Conceptual API (internal)
let page = page_cache.pin(page_id)?;
// ... use the page ...
page_cache.unpin(page_id)?;
```

## Checksums

Every page has a CRC32 checksum in its header. The checksum is computed over the page data (excluding the checksum field itself) and verified on every page load. If a checksum mismatch is detected, MongoLite attempts to recover the page from the WAL.

## Page Compaction

Over time, data pages can become fragmented due to deletions and updates. MongoLite performs **page compaction** during checkpoints:

1. Documents are rearranged to eliminate gaps.
2. Excess free space is consolidated.
3. If a page becomes entirely empty, it is added to the free list.
