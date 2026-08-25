# File Format

MongoLite stores the entire database in a single `.mongolite` file. This document describes the on-disk structure of that file.

## Overview

The `.mongolite` file is organized as a sequence of fixed-size pages. The first page (page 0) is always the **file header**, which contains metadata about the database. Subsequent pages are allocated by the storage engine for indexes, documents, and internal bookkeeping.

```
┌─────────────────────────────────────────────────┐
│  Page 0: File Header                            │
├─────────────────────────────────────────────────┤
│  Page 1: Root Index (default _id index)        │
├─────────────────────────────────────────────────┤
│  Page 2: Collection Catalog                     │
├─────────────────────────────────────────────────┤
│  Page 3+: Data pages, index pages, free list   │
├─────────────────────────────────────────────────┤
│  ...                                            │
└─────────────────────────────────────────────────┘
```

## File Header

The file header occupies the first 4096 bytes of the file (the first page). It contains:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 16 | `magic` | Magic bytes: `MONOLITE\x00\x00\x00\x00\x00\x00\x00` |
| 16 | 4 | `version` | File format version (currently `1`) |
| 20 | 4 | `page_size` | Size of each page in bytes (default: 4096) |
| 24 | 8 | `page_count` | Total number of pages in the file |
| 32 | 8 | `root_index_page` | Page ID of the root B+tree index |
| 40 | 8 | `catalog_page` | Page ID of the collection catalog |
| 48 | 8 | `free_list_head` | Page ID of the first free page (0 if none) |
| 56 | 8 | `wal_offset` | Byte offset to the WAL region |
| 64 | 32 | `uuid` | Unique database identifier (UUID v4) |
| 96 | 3998 | `reserved` | Reserved for future use (zero-filled) |

## Page Structure

Every page in the file shares a common header:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 2 | `page_type` | Type identifier (see below) |
| 2 | 2 | `flags` | Page flags (dirty, pinned, etc.) |
| 4 | 4 | `page_id` | Unique page identifier |
| 8 | 4 | `checksum` | CRC32 checksum of page contents |
| 12 | varies | `data` | Page-type-specific data |

### Page Types

| Value | Name | Description |
|-------|------|-------------|
| `0x01` | `PAGE_HEADER` | File header (page 0 only) |
| `0x02` | `PAGE_INDEX_LEAF` | B+tree leaf node containing index entries |
| `0x03` | `PAGE_INDEX_INTERNAL` | B+tree internal node containing child pointers |
| `0x04` | `PAGE_DATA` | Raw document data |
| `0x05` | `PAGE_CATALOG` | Collection metadata catalog |
| `0x06` | `PAGE_OVERFLOW` | Overflow page for large documents |
| `0x07` | `PAGE_FREE` | Free/available page |

## Page Size

The default page size is **4096 bytes** (4 KiB), matching the typical OS page size. This can be configured at database creation time to 8192, 16384, or 32768 bytes for workloads that benefit from larger pages.

## Free List

When pages are deleted (e.g., after document removal), they are added to a singly-linked free list. The head of this list is stored in the file header. When the storage engine needs a new page, it first checks the free list before extending the file.

## File Growth

The `.mongolite` file grows by allocating new pages at the end. The storage engine uses a **doubling strategy** for file extensions: when the file needs to grow, it doubles the current size up to a threshold, then switches to fixed-size increments to avoid excessive allocation.

## Future Extensions

Planned additions to the file format:

- **Encryption header** — Support for AES-256-GCM encryption at rest.
- **Compression flags** — Per-page compression using zstd.
- **Sharding metadata** — For future distributed MongoLite deployments.
