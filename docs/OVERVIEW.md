# MongoLite Overview

## Concept

MongoLite is a **serverless, single-file** document database written in
pure Rust. It exposes enough of the MongoDB query language (insert, find,
count, update, delete, basic aggregation operators) so that existing BSON
tooling and drivers can be reused in a local-embedded scenario.

The entire database lives in **one `.mongoLite` file** on disk — no daemon,
no network port, and no background threads. Every client operation maps 1:1
to a synchronous file read/write, making the concurrency story simple and
the failure modes easy to reason about.

## Why

- **Edge/IoT**: A 500 KB library footprint that embeds directly into a larger
  Rust binary or ships via FFI to C/C++/Python.
- **Fast tests**: Spin up an isolated MongoLite instance in `<100 ms` from a
  temporary directory. No Docker, no `mongod` startup delay.
- **Migration staging**: Develop MongoDB schema logic locally, then push to a
  real cluster without changing code paths for basic queries.

## High-level Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                          mongolite                             │
├─────────────────────────────────────────────────────────────────┤
│  public API  :  Database ──► Collection ──► (Cursor/Iterator)   │
│                                                     │          │
│  persistence:  Storage Engine (File, MMIO, WAL, BTree)          │
│                ▲  uses                         ▲ writes via     │
│                │   Page Pool (LRU)             │ WAL            │
│                │                                │                │
│  query layer :  QueryMatcher (BSON → bool)                     │
│                                                     │          │
│  CLI binary  :  crates/mongolite-cli (REPL)                     │
│  FFI         :  crates/mongolite-ffi (C ABI)                    │
└─────────────────────────────────────────────────────────────────┘
```

1. **Memory-mapped I/O** – All file access goes through `memmap2::MmapMut`.
2. **Write-ahead log** – Mutations are appended to the `.mongoLite-wal`
   file *before* touching the main database pages. On next startup, an
   incomplete WAL record triggers automatic replay.
3. **B+Tree catalog** – A global B-Tree maps collection names to B+Tree
   root page IDs; each collection is stored in its own B+Tree index
   `<_id-bytes> -> <bson-doc-bytes>`.
4. **Lazy collection creation** – Collections appear on first insert;
   metadata B+Tree roots are created on demand and cached forever in the
   catalog.

## Repository Layout

```
mongolite/                 ← repo root
├── Cargo.toml             ← workspace manifest (3 crate members)
│
├── crates/
│   ├── mongolite/         ← core library (.crate)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs          # Public re-exports
│   │       ├── db.rs           # Database, collection lookup
│   │       ├── collection.rs   # CRUD methods
│   │       ├── cursor.rs       # Iterator/Projection/Sort
│   │       ├── query.rs        # BSON filter evaluation
│   │       ├── error.rs        # thiserror error enum
│   │       ├── cli.rs          # REPL (unused in CLI bin!)
│   │       ├── document/       # BSON wrapper + ObjectId
│   │       └── storage/        # File, Page, Allocator, BTree, WAL, Pool
│   │
│   ├── mongolite-cli/
│   │   ├── Cargo.toml
│   │   └── src/main.rs          # Console REPL wrapper binary
│   │
│   └── mongolite-ffi/
│       ├── Cargo.toml
│       ├── src/lib.rs
│       └── mongolite.h          # C ABI header
│
├── benches/bench.rs             # Criterion-style benchmarks
├── tests/                        # Integration tests (TBD)
└── docs/
    ├── OVERVIEW.md  ← you are here
    ├── CHANGELOG.md
    ├── API.md
    ├── architecture/
    ├── storage/
    ├── query/
    └── api/
```

## Dependencies Snapshot

| Crate          | Purpose                      | Version |
|----------------|------------------------------|---------|
| `bson`         | BSON encoding/decoding     | 2.x     |
| `serde_json`   | JSON ↔ BSON parsing          | 1.x     |
| `crc32fast`    | Fast checksum / integrity    | 1.x     |
| `memmap2`      | Memory-mapped file I/O       | 0.9     |
| `thiserror`    | Ergonomic derive(Error)      | 2.x     |
| `regex`        | $regex operator matching     | 1.x     |

Optional features:

- `mmap` *(default)* – Enables the `memmap2` dependency for memory-mapped I/O.
- `wasm` – Strips out anything requiring OS-level file descriptors (future use).

## Quick Start (Rust)

```toml
# Cargo.toml
[dependencies]
mongolite = { path = "../path/to/mongolite" }
```

```rust
use mongolite::Database;
use bson::doc;

let mut db = Database::create("test.mongolite")?;
let mut users = db.collection("users");

users.insert_one(doc! { "name": "Alice", "age": 30 })?;
let found = users.find(doc! { "age": { "$gt": 25 }})?;
println!("{:?}", found);

// Flush and close
db.flush()?;
```

## Quick Start (C via FFI)

```c
#include "mongolite.h"

void *db = mongolite_create("test.mongolite");
mongolite_insert(db, "users", "{\"name\":\"Alice\",\"age\":30}", NULL, 0);
char buf[1024];
int len = mongolite_find(db, "users", "{\"age\":{\"$gt\":25}}", buf, sizeof(buf));
mongolite_close(db);
```

## License

Apache-2.0 — see `LICENSE` file in crate root.
