# Pages

MongoLite uses a **fixed-size page abstraction** for all storage operations.
All file offsets are rounded up to the nearest page boundary, and physical I/O
happens in page-sized chunks.

## Constants

| Constant            | Value | Meaning                          |
|---------------------|-------|----------------------------------|
| `DEFAULT_PAGE_SIZE` | `4096`| Default page size in bytes.      |
| `PageHeader::SIZE`  | `8`   | Fixed size of every page header. |

Page sizes smaller than 512 bytes or not powers of two are rejected at file
creation time.

## Page Header Structure

The first 8 bytes of every page contain control data.

| Byte Range | Field       | Type  | Description                            |
|------------|-------------|-------|----------------------------------------|
| `[0]`      | `page_type` | `u8`  | See next section.                      |
| `[1]`      | `flags`     | `u8`  | Reserved for future flags (unused for now). |
| `[2..4]`   | `padding`   | `u16` | Unused, set to zero.                   |
| `[4..8]`   | `checksum`  | `u32` | CRC32 of everything from byte `8` onwards. |

## Page Types

| Raw Value | Symbol         | Description                        |
|-----------|----------------|------------------------------------|
| `0x00`    | `Free`         | Part of the free list              |
| `0x01`    | `BTreeLeaf`    | Leaf node in B+Tree                |
| `0x02`    | `BTreeInternal`| Internal routing node              |
| `0x03`    | `Overflow`     | Overflow record for big payloads   |
| `0x04`    | `Catalog`      | Catalog/metadata page              |

## Reading and Writing

- Immutable reads happen through `File::page(id)` which returns `&[u8]`.
- Mutable writes go through `File::page_mut(id)` which returns `&mut [u8]`,
  marks the page as dirty, and returns mutable access.
- After mutation, the caller must invoke `PageMut::compute_checksum()` and
  `PageMut::write_header()` to persist integrity data.

## Integrity Checks

On every call to `Page::new(data)`:

- The raw page type is validated against known types.
- The CRC32 checksum stored in the header is compared to one computed from the
  page payload.
- Mismatches generate `Error::Corrupted`.

Checksums are **not verified on normal operation** inside the hot path, but can
be explicitly checked via `Page::validate_checksum()`.

---

See also: [B+Tree internals](02-btree.md) | [WAL format](03-wal.md)
