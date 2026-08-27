# MongoLite

A serverless, single-file, MongoDB-compatible document database written in Rust.

## Status

Early development — v0.1.0 in progress.

## Features

- Single-file storage — entire database in one `.mongolite` file
- BSON document format — MongoDB-compatible encoding
- B+tree indexes — efficient document retrieval
- WAL crash safety — write-ahead log for durability
- Memory-mapped I/O — high-performance reads/writes
- FFI bindings — C ABI for embedding in other languages

## Quick Start

```rust
use mongolite::Database;
use bson::doc;

let mut db = Database::create("mydb.mongolite")?;
let mut users = db.collection("users");

users.insert_one(doc! { "name": "Alice", "age": 30 })?;
let found = users.find(doc! { "age": { "$gt": 25 } })?;
println!("{:?}", found);

db.flush()?;
```

## CLI

```bash
cargo run -p mongolite-cli --release -- ./test.db
mongolite> insert users {"name": "Alice", "age": 30}
mongolite> find users {"age": {"$gt": 25}}
```

## Documentation

See the `docs/` directory for full documentation:
- `docs/OVERVIEW.md` — project overview
- `docs/API.md` — API reference
- `docs/architecture/` — storage engine internals
- `docs/storage/` — pages, B+Tree, WAL
- `docs/query/` — query operators and projection

## License

Apache-2.0 — see `LICENSE` file.

## Contributing

See `CONTRIBUTING.md` for guidelines.
