# WAL (Write-Ahead Log)

MongoLite ships with a minimal but robust **write-ahead log (WAL)** to
guarantee crash safety without locking the main database file.

## Companion File

Every `.mongoLite` database has an associated WAL file:

```
mydb.mongolite       ← main DB file (memory-mapped)
mydb.mongolite-wal   ← WAL log file (also memory-mapped)
```

The WAL is created automatically on first write if it does not already exist.

## WAL Record Format

Each WAL record consists of:

```
┌─ WalRecordHeader ─────────────────────┐
│ page_id       : u32 │ target page in DB file |
│ data_length   : u32 │ payload size in bytes   |
│ checksum      : u32 │ CRC32 of payload        |
└───────────────────────────────────────┘
┌─ Payload (variable length) ───────────┐
│ Raw page bytes to restore on replay   |
│ Length == data_length                 |
└───────────────────────────────────────┘
```

- The WAL is written sequentially from offset `WalHeader::SIZE` upwards.
- A zero-length `data_length` terminates the stream — marks end of log.
- Payloads are full-page images (not diffs).

## WAL Header (`WalHeader`)

At offset `0` of the WAL file:

| Field                  | Type | Notes                          |
|------------------------|------|--------------------------------|
| `magic`                | u32  | `0x57414C21` ("WAL!")          |
| `page_size`            | u32  | Must match page size of DB     |
| `last_committed_pages` | u32  | Page count at last checkpoint  |
| `checksum`             | u32  | CRC32 of preceding fields      |

## Lifecycle

1. **Open**: MongoLite opens the WAL (or creates one if missing).
2. **Append**: Before flushing any changed pages to the main DB file,
   `Wal::append_page(page_id, data)` copies the full page image into the WAL.
3. **Checkpoint**: Once the DB is safely synced, `Wal::checkpoint()` resets the
   write offset to `WalHeader::SIZE`, truncating future appends.
4. **Replay**: On next open, if the WAL exists and contains records newer than
   the last checkpoint, they are replayed into the DB file via
   `Wal::replay()`.

## Flush & Sync Behavior

| Action          | What Happens                             |
|------------------|------------------------------------------|
| `Wal::append_page`   | Appends a record to the in-memory mmap buffer. |
| `Database::flush`    | Calls `File::flush()` which triggers `mmap.flush()` internally. |
| `Wal::checkpoint`    | Updates `last_committed_pages` field and zeroes the write offset. |
| `Drop<Wal>`          | Implicitly calls `File::flush()` to sync remaining log entries. |

## Error Handling

| Scenario                        | Error Returned                |
|----------------------------------|-------------------------------|
| Invalid magic number             | `Error::Wal("invalid WAL magic")` |
| CRC mismatch during replay       | `Error::Wal("WAL record checksum mismatch")` |
| Corrupt header during open       | `Error::Wal("WAL header too short")` |

## Limitations

- WAL records are **full-page images**, not diffs. Therefore, large documents
  cause significant I/O amplification — this may change in future versions
  with differential logging.
- The WAL is truncated (not archived). It acts purely as a transient log;
  once checkpointed, prior history is lost.
- Only **single-file** databases are supported. WAL replay assumes the same
  file path exists.

---

Related: [Page Format](01-pages.md) | [B+Tree Index](02-btree.md)
