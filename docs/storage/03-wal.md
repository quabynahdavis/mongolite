# Write-Ahead Log (WAL)

The Write-Ahead Log (WAL) is MongoLite's mechanism for ensuring durability and crash recovery. All modifications are written to the WAL before they are applied to the main data pages.

## Why WAL?

Without a WAL, a crash during a write could leave the database in an inconsistent state. The WAL solves this by:

1. Recording every modification before it is applied.
2. Allowing the system to replay or undo modifications after a crash.
3. Guaranteeing that committed data is never lost.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Write Operation                    │
├─────────────────────────────────────────────────────┤
│  1. Serialize modification to WAL record            │
│  2. Append record to WAL buffer                     │
│  3. fsync WAL buffer to disk                        │
│  4. Apply modification to memory-mapped pages       │
│  5. Mark pages as dirty                             │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│                  Checkpoint Process                  │
├─────────────────────────────────────────────────────┤
│  1. Flush all dirty pages to disk                   │
│  2. Record checkpoint marker in WAL                 │
│  3. Truncate WAL up to checkpoint                   │
└─────────────────────────────────────────────────────┘
```

## WAL File Structure

The WAL is stored in a separate region of the `.mongolite` file (or in a separate `.wal` file in future versions). It consists of a sequence of **records**:

```
┌─────────────────────────────────────────────────────┐
│ WAL Header                                          │
│   - magic number                                    │
│   - version                                         │
│   - last checkpoint LSN                             │
├─────────────────────────────────────────────────────┤
│ Record 1                                            │
│   - LSN (Log Sequence Number)                       │
│   - transaction ID                                  │
│   - operation type                                  │
│   - page ID                                         │
│   - data length                                     │
│   - before-image (for undo)                         │
│   - after-image (for redo)                          │
│   - CRC32 checksum                                  │
├─────────────────────────────────────────────────────┤
│ Record 2                                            │
│   ...                                               │
├─────────────────────────────────────────────────────┤
│ Checkpoint Record                                   │
│   - checkpoint LSN                                  │
│   - active transactions                             │
│   - dirty page table                                │
└─────────────────────────────────────────────────────┘
```

## Log Sequence Numbers (LSN)

Every WAL record is assigned a monotonically increasing **Log Sequence Number**. The LSN is used to:

- Order records chronologically.
- Determine which records need to be replayed during recovery.
- Track the progress of checkpoints.

## Record Types

| Type | Description |
|------|-------------|
| `INSERT` | New document inserted into a page |
| `UPDATE` | Existing document modified |
| `DELETE` | Document removed from a page |
| `PAGE_ALLOC` | New page allocated |
| `PAGE_FREE` | Page deallocated |
| `CHECKPOINT` | Checkpoint marker |
| `TXN_BEGIN` | Transaction start |
| `TXN_COMMIT` | Transaction commit |
| `TXN_ABORT` | Transaction abort |

## Write Path

When a write operation occurs:

1. **Serialize** the modification into a WAL record containing:
   - The page being modified.
   - The **before-image** (original data for undo).
   - The **after-image** (new data for redo).

2. **Append** the record to the WAL buffer in memory.

3. **fsync** the WAL buffer to disk (guarantees durability).

4. **Apply** the modification to the memory-mapped page.

5. **Mark** the page as dirty in the page cache.

## Checkpointing

Checkpoints flush dirty pages to disk and allow WAL truncation:

### Checkpoint Process

1. Write a `CHECKPOINT` record to the WAL.
2. Flush all dirty pages to the main data file.
3. Write a `CHECKPOINT_END` record.
4. Truncate the WAL up to the checkpoint LSN.

### Checkpoint Triggers

| Trigger | Condition |
|---------|-----------|
| Time-based | Every 60 seconds |
| WAL size | When WAL exceeds 100 MiB |
| Dirty pages | When > 25% of pages are dirty |
| Manual | `db.checkpoint()?` |

## Crash Recovery

On startup, MongoLite checks if the database was cleanly shut down. If not, it performs crash recovery:

### Recovery Process

1. **Find the last checkpoint** in the WAL.
2. **Redo phase** — Replay all WAL records after the checkpoint (forward recovery).
3. **Undo phase** — Roll back any uncommitted transactions (backward recovery).
4. **Verify** — Check page checksums to ensure consistency.

### Redo Phase

Starting from the checkpoint LSN, scan forward through the WAL:

```
Checkpoint LSN: 100
Records:  [101] [102] [103] [104] [105]
           ─── Redo all records ──▶
```

For each record, reapply the after-image to the corresponding page.

### Undo Phase

Scan backward from the end of the WAL:

```
Records:  [101] [102] [103] [104] [105]
           ◀── Undo uncommitted ───
```

For each uncommitted transaction, apply the before-image to reverse the modification.

## WAL Size Management

The WAL grows as records are appended. To prevent unbounded growth:

- **Truncation** — After a checkpoint, old records are removed.
- **Rotation** — When the WAL exceeds a maximum size, a new WAL segment is started.
- **Compression** — (Future) WAL records can be compressed with zstd.

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `wal_enabled` | `true` | Enable/disable WAL (disabling risks data loss) |
| `wal_sync_interval_ms` | 100 | Maximum time between WAL fsyncs |
| `wal_max_size_mb` | 100 | Maximum WAL size before forced checkpoint |
| `checkpoint_interval_s` | 60 | Time between automatic checkpoints |

## Trade-offs

| Setting | Durability | Performance |
|---------|------------|-------------|
| `fsync` every write | Highest | Slowest |
| `fsync` every 100ms | High | Moderate |
| `fsync` every 60s | Moderate | Fastest |
| No WAL (in-memory) | None | Fastest |

The default configuration (`fsync` every 100ms) provides a good balance between durability and performance for most workloads.
