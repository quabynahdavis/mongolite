# MongoLite Documentation

Welcome to the MongoLite documentation. MongoLite is a serverless, single-file, MongoDB-compatible document database written in Rust.

## Status

**Version:** 0.1.0 — Early development

## Documentation Index

| Directory | Description |
|-----------|-------------|
| [architecture](./architecture/) | Core architectural concepts: file format, storage engine, and document model |
| [api](./api/) | Public API reference: Database, Collection, and Cursor interfaces |
| [storage](./storage/) | Storage internals: pages, B+tree indexes, and write-ahead log |
| [query](./query/) | Query language: operators, updates, and projections |

## Quick Start

```rust
use mongolite::Database;

// Open (or create) a single-file database
let db = Database::open("mydb.mongolite")?;

// Get a collection
let users = db.collection("users");

// Insert a document
users.insert_one(doc! { "name": "Alice", "age": 30 })?;

// Query documents
let results = users.find(doc! { "age": { "$gte": 25 } })?;
```

## Project Structure

```
mongolite/
├── crates/
│   ├── mongolite/        # Core library
│   ├── mongolite-cli/    # CLI tool
│   └── mongolite-ffi/    # C bindings
├── tests/                # Integration tests
├── benches/              # Benchmarks
└── docs/                 # This documentation
```

## License

Apache-2.0
