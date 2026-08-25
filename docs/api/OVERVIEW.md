# API Documentation

This folder contains the public API reference for MongoLite. The API is designed to be familiar to anyone who has used MongoDB drivers in other languages.

## Contents

| File | Description |
|------|-------------|
| [01-database](./01-database.md) | The `Database` type: opening, closing, and managing databases |
| [02-collection](./02-collection.md) | The `Collection` type: CRUD operations on document collections |
| [03-cursor](./03-cursor.md) | The `Cursor` type: iterating over query results |

## Core Types

MongoLite's public API is built around three primary types:

```
Database
  └── Collection
        └── Cursor
```

- **Database** — Represents a single `.mongolite` file. Provides methods to create, list, and drop collections.
- **Collection** — Represents a group of documents. Provides CRUD operations (insert, find, update, delete).
- **Cursor** — An iterator over query results. Supports sorting, skipping, and limiting.

## Error Handling

All API methods return `Result<T, MongoLiteError>`. The error type covers:

| Error | Description |
|-------|-------------|
| `IoError` | Underlying I/O failure (disk full, permission denied, etc.) |
| `BsonError` | BSON encoding/decoding failure |
| `NotFound` | Requested resource does not exist |
| `DuplicateKey` | Unique index violation |
| `InvalidArgument` | Invalid parameter supplied |
| `InternalError` | Unexpected internal state |

## Thread Safety

- `Database` is `Send + Sync` — safe to share between threads.
- `Collection` is `Send + Sync` — multiple threads can operate on the same collection.
- `Cursor` is `Send` but not `Sync` — each cursor should be used from a single thread.

## Quick Example

```rust
use mongolite::Database;
use bson::doc;

fn main() -> Result<(), mongolite::Error> {
    // Open a database
    let db = Database::open("app.mongolite")?;

    // Get a collection
    let users = db.collection("users");

    // Insert
    users.insert_one(doc! { "name": "Alice", "age": 30 })?;

    // Find
    let cursor = users.find(doc! { "age": { "$gte": 18 } })?;
    for doc in cursor {
        println!("{:?}", doc?);
    }

    Ok(())
}
```
