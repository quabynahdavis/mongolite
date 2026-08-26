# Architecture Domain Overview

This folder explains the design principles and internal mechanics of MongoLite.

## Contents

| File                        | Topic                              |
|-----------------------------|------------------------------------|
| `01-file-format.md`         | On-disk layout, headers, pages     |
| `02-storage-engine.md`      | B+Tree, WAL, allocator, pool       |
| `03-document-model.md`      | BSON model, ObjectId format        |

## Audience

- Contributors working on core storage logic.
- Developers integrating via FFI who need insight into failure modes.
- Researchers evaluating embedded storage tradeoffs.

## Prerequisites

Basic familiarity with:
- Rust ownership model
- BSON encoding
- B+Tree data structures
- Memory-mapped I/O concepts

## See Also

- [API Reference](../api/OVERVIEW.md)
- [Query Language](../query/OVERVIEW.md)
