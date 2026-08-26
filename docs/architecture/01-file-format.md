# File Format

## Disk Layout

A MongoLite database is stored in a **single memory-mapped file**
named `*.mongolite`. The file is divided into fixed-size **pages** of
`4096` bytes (default, must be a power of 2 ≥ 512).

```
byte 0  ┌─────────────────────────────┐
         │        File Header          │  40 bytes  (fixed)
         ├─────────────────────────────┤
         │                             │
         │       Page 0               │  4096 bytes (root / catalog)
         │                             │
         ├─────────────────────────────┤
         │       Page 1               │  4096 bytes
         ├─────────────────────────────┤
         │       Page 2               │
         ├─────────────────────────────┤
         │           …                │
         └─────────────────────────────┘
```

The file grows by **doubling** its size each time the allocator runs out of
free pages (from 1 page → 2 → 4 → 8 → …). Growth is triggered by
`File::grow()`, which maps the new (larger) file and zeros out the new
half.

## File Header (`FileHeader`)

Written to the first bytes of page 0, the header holds all metadata
required to open the database without scanning pages.

| Offset | Field                | Type     | Notes |
|--------|----------------------|----------|-------|
| 0      | `magic`              | `[u8;4]` | `b"MDOC"` — validates file identity. |
| 4      | `version_major`      | `u16`    | Currently `0`. |
| 8      | `version_minor`      | `u16`    | Currently `1`. |
| 12     | `page_size`          | `u32`    | Default `4096`. |
| 16     | `total_pages`        | `u32`    | Current page count. |
| 20     | `page_count_at_checkpoint` | `u32` | Last clean page count (WAL checkpoint). |
| 24     | `free_list_head`     | `u32`    | Page ID of head of free list, or `0`. |
| 28     | `catalog_root_page`  | `u32`    | B-Tree root page of the **catalog**. |
| 32     | `wal_magic`          | `u32`    | WAL presence marker (reserved). |
| 36     | `document_count`     | `u64`    | Total docs across all collections. |
| 40     | `checksum`           | `u32`    | CRC32 of bytes `0..36`. |

`FileHeader::compute_checksum()` excludes the `checksum` field itself so
that a simple `==` comparison detects corruption after load.

## Page Layout

Every page begins with an 8-byte `PageHeader`:

```
offset 0  │ page_type (u8)   0x00=Free 0x01=Leaf 0x02=Internal …
offset 1  │ flags (u8)
offset 2  │ padding (u16)
offset 4  │ checksum (u32)   CRC32 of payload bytes 8..end
offset 8  │ payload begins…
```

### Page Types

| Value | Name         | Used For |
|-------|--------------|----------|
| `0x00` | `Free`       | Recycled pages in the free list. |
| `0x01` | `BTreeLeaf`  | B-Tree leaf nodes — stores key/value entries. |
| `0x02` | `BTreeInternal` | B-Tree internal nodes — routing keys + child pointers. |
| `0x03` | `Overflow`   | Large values that don't fit in a single leaf page. |
| `0x04` | `Catalog`    | Reserved; catalog lives in a normal BTreeLeaf/Internal today. |

## B-Tree Leaf Page Layout

```
[page_type=1][num_keys:u32][next_leaf_page:u32][entry_0][entry_1]…[entry_N]
              ▲ 4 bytes            ▲ 4 bytes
```

Each **entry** is a packed, variable-length record:

```
[key_len:u32][key_bytes][value_len:u32][value_bytes]
  4 bytes    key_len    4 bytes      value_len
```

No separators or alignment are used between entries — offsets are computed
by reading `key_len` and `value_len` in sequence.

## B-Tree Internal Page Layout

```
[page_type=0][num_keys:u32][child_0:u32][key_0][child_1:u32][key_1][child_2]…
```

The invariant is `num_keys = num_children - 1`. Keys are sorted in
ascending order; all keys in the subtree pointed to by `child_i` are
`≤ key_i` and `> key_{i-1}`.

## Overflow Pages

Values (serialised BSON documents) larger than `(page_size - 8 - overhead)`
are stored in one or more linked overflow pages. The leaf entry stores only
the first page ID; subsequent pages are chained via `END_OF_CHAIN`
(`0xFFFFFFFF`).

## Free Page List

Freed pages are chained via a singly-linked list stored **inline in the
page payload** (first 4 bytes of the freed page point to the next free
page). The head of the chain is stored in `FileHeader.free_list_head`.
Allocation pops from the head; freeing pushes to the head. When the free
list is empty, a new page is allocated from the end of the file.

## WAL Companion File

A `.mongolite-wal` file (created alongside the main file) holds the
write-ahead log. Its layout and semantics are described in
[docs/storage/03-wal.md](../storage/03-wal.md).
