# Architecture Documentation

This folder covers the core architectural concepts behind MongoLite. It explains how data is structured on disk, how the storage engine manages reads and writes, and how the document model provides MongoDB compatibility.

## Contents

| File | Description |
|------|-------------|
| [01-file-format](./01-file-format.md) | The `.mongolite` single-file format: header, page layout, and file structure |
| [02-storage-engine](./02-storage-engine.md) | How the storage engine works: memory-mapped I/O, page cache, and read/write paths |
| [03-document-model](./03-document-model.md) | The BSON document model and how it maps to MongoDB's type system |

## Design Principles

MongoLite is built around these core design choices:

1. **Single-file storage** — The entire database lives in one `.mongolite` file, making it portable and easy to back up.
2. **BSON compatibility** — Documents use MongoDB's BSON encoding, ensuring interoperability.
3. **B+tree indexes** — Provides efficient range queries and ordered traversal.
4. **WAL-based durability** — A write-ahead log guarantees crash recovery without data loss.
5. **Memory-mapped I/O** — Leverages the OS virtual memory system for high-performance access.

## Reading Order

For a thorough understanding of MongoLite's architecture, read the documents in order:

1. Start with [01-file-format.md](./01-file-format.md) to understand the on-disk layout.
2. Continue with [02-storage-engine.md](./02-storage-engine.md) to learn how data moves between disk and memory.
3. Finish with [03-document-model.md](./03-document-model.md) to understand document representation.
