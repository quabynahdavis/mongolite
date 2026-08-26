# MongoLite API Documentation

This document provides a high‑level reference for the public API of the **mongolite** crate. It follows the structure of the source tree and describes the most important types, their responsibilities, and typical usage patterns.

---

## Table of Contents

1. [Top‑Level Exports](#top-level-exports)
2. [Database (`db` module)](#database-db-module)
3. [Collection (`collection` module)](#collection-collection-module)
4. [Document (`document` module)](#document-document-module)
5. [Query (`query` module)](#query-query-module)
6. [Cursor (`cursor` module)](#cursor-cursor-module)
7. [Storage Engine (`storage` module)](#storage-storage-module)
8. [Error Handling (`error` module)](#error-error-module)
9. [Command‑Line Interface (`cli` module)](#cli-cli-module)
10. [Examples](#examples)
---

## 1. Top‑Level Exports

```rust
pub use db::Database;
pub use error::{Error, Result};
```

- **`Database`** – entry point for creating/opening a database file.
- **`Error` / `Result<T>`** – common error type used throughout the crate.

All other modules are accessed through the `Database` methods or via the CLI.

---

## 2. Database (`db` module)

### Struct `Database`
```rust
pub struct Database {
    // internal fields hidden from the public API
}
```

#### Key Methods
- `pub fn create<P: AsRef<Path>>(path: P) -> Result<Self>` – create a new, empty database file.
- `pub fn open<P: AsRef<Path>>(path: P) -> Result<Self>` – open an existing database.
- `pub fn collection(&self, name: &str) -> Collection` – obtain a handle to a collection (creates it lazily if missing).
- `pub fn list_collections(&self) -> Result<Vec<String>>` – retrieve the names of all collections.
- `pub fn drop_collection(&mut self, name: &str) -> Result<()>` – permanently delete a collection.

### Behaviour
- The underlying storage is a single file (`*.mongolite`).
- Operations are atomic; the storage engine uses a write‑ahead log (WAL) to guarantee durability.
- Collections are stored as B‑Tree roots within the file.

---

## 3. Collection (`collection` module)

### Struct `Collection`
```rust
pub struct Collection {
    // private fields
}
```

#### Core Operations
- `pub fn insert_one(&self, doc: Document) -> Result<InsertResult>` – inserts a single document; returns the generated `_id`.
- `pub fn insert_many(&self, docs: Vec<Document>) -> Result<InsertManyResult>` – bulk insert.
- `pub fn find(&self, filter: Option<Document>) -> Result<Vec<Document>>` – simple find that returns a vector of matching documents.
- `pub fn find_one(&self, filter: Option<Document>) -> Result<Option<Document>>` – returns the first matching document.
- `pub fn count(&self, filter: Option<Document>) -> Result<usize>` – count matching docs.
- `pub fn delete_one(&self, filter: Document) -> Result<DeleteResult>` – delete first match.
- `pub fn delete_many(&self, filter: Document) -> Result<DeleteResult>` – delete all matches.

#### Indexes (future work)
The current implementation does **not** expose a public indexing API; all lookups are linear scans or B‑Tree range scans provided by the storage layer.

---

## 4. Document (`document` module)

MongoLite stores data as BSON documents.

### Types
- `pub type Document = bson::Document;` – re‑export of the `bson` crate’s `Document` type.
- `pub struct ObjectId(pub [u8; 12]);` – wrapper around a 12‑byte identifier used for the `_id` field.

### Helpers
- `pub fn new_object_id() -> ObjectId` – generates a new, random object id.
- `impl From<ObjectId> for Document` – automatically inserts `_id` when converting.

---

## 5. Query (`query` module)

The query module implements a tiny subset of MongoDB’s query language. It parses a JSON document into an internal predicate structure that the storage engine evaluates.

### Public Functions
- `pub fn parse_filter(filter: &Document) -> Result<Predicate>` – converts a BSON filter document into a `Predicate`.
- `pub fn evaluate(predicate: &Predicate, doc: &Document) -> bool` – tests whether a document satisfies the predicate.

### Predicate Enum (simplified)
```rust
pub enum Predicate {
    Eq(String, Bson),          // field == value
    Gt(String, Bson),          // field > value
    Lt(String, Bson),          // field < value
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    Not(Box<Predicate>),
    // …more operators could be added later
}
```

---

## 6. Cursor (`cursor` module)

A cursor is a lightweight iterator over a result set. It is primarily used internally by `Collection::find` but can be exposed for streaming large results.

### Struct `Cursor`
```rust
pub struct Cursor {
    // internal state, current page, position, etc.
}
```

#### Main API
- `pub fn next(&mut self) -> Option<Result<Document>>` – fetch the next document.
- `pub fn into_iter(self) -> impl Iterator<Item = Result<Document>>` – consume the cursor into a Rust iterator.

---

## 7. Storage Engine (`storage` module)

The storage engine lives under `mongolite::storage` and implements a simple file‑based B‑Tree with a write‑ahead log. The most relevant public items are:

### Core Types
- `pub struct Page` – fixed‑size block that holds a slice of B‑Tree nodes or leaf values.
- `pub struct BTree` – the B‑Tree index for a collection. Handles node splits, merges, and search.
- `pub struct Wal` – write‑ahead log; each modification appends a record that can be replayed on crash recovery.
- `pub struct Allocator` – manages free pages inside the file.

### Public Functions (used indirectly)
- `pub fn init(path: &Path) -> Result<Storage>` – initialize a new storage file.
- `pub fn open(path: &Path) -> Result<Storage>` – open an existing file.
- `pub fn recover(storage: &mut Storage) -> Result<()>` – replay the WAL to bring the database to a consistent state after an abnormal shutdown.

The storage module is deliberately **not** part of the public API; `Database` and `Collection` expose higher‑level methods.

---

## 8. Error Handling (`error` module)

### Enum `Error`
```rust
pub enum Error {
    Io(std::io::Error),
    Serde(serde_json::Error),
    InvalidQuery(String),
    NotFound(String),
    // …other variants as needed
}
```

All library functions return `Result<T, Error>` (type‑aliased as `crate::Result<T>`).

### Implementations
- `impl std::fmt::Display for Error`
- `impl std::error::Error for Error`
- `impl From<std::io::Error> for Error`
- `impl From<serde_json::Error> for Error`

---

## 9. Command‑Line Interface (`cli` module)

The REPL located in `src/cli.rs` is a tiny interactive wrapper around a `Database`. It parses user input, forwards commands to the underlying collection methods, and prints results. The public entry point is the `Cli` struct:

```rust
pub struct Cli {
    db: Database,
}
```

Key method:
- `pub fn run(&mut self) -> Result<()>` – starts the prompt loop.

Supported commands (see `print_help`):
- `tables` – list collections.
- `insert <coll> <json>` – insert a document.
- `find <coll> [filter]` – retrieve matching documents.
- `count <coll> [filter]` – count documents.
- `delete <coll> <filter>` – delete matching documents.
- `stats` – show per‑collection document counts.
- `help` / `quit`.

---

## 10. Examples

### Basic usage (library)
```rust
use mongolite::{Database, Result};
use bson::doc;

fn main() -> Result<()> {
    // Create a new database file (or open an existing one)
    let db = Database::create("mydb.mongolite")?;

    // Work with a collection named "users"
    let users = db.collection("users");

    // Insert a document
    let id = users.insert_one(doc! { "name": "Alice", "age": 30 })?.inserted_id;
    println!("Inserted document with _id: {}", id);

    // Find all users older than 25
    let filter = doc! { "age": { "$gt": 25 } };
    let results = users.find(Some(filter))?;
    for user in results {
        println!("User: {}", serde_json::to_string_pretty(&user)?);
    }

    Ok(())
}
```

### Using the CLI
```bash
$ cargo run -p mongolite-cli --release
MongoLite CLI v0.1.0
Type 'help' for commands, 'quit' to exit.

mongolite> insert users {"name": "Bob", "age": 28}
Inserted _id: 5f1d9c7a8b9e4a2b4c3d6f00

mongolite> find users {"age": {"$gt": 20}}
{
  "_id": "5f1d9c7a8b9e4a2b4c3d6f00",
  "name": "Bob",
  "age": 28
}
1 document(s) found.
```

---

## 11. Extending MongoLite

The library is deliberately minimal; additional features can be built on top:
- **Indexes** – expose a public `Index` API that creates B‑Tree secondary structures.
- **Advanced Queries** – add support for `$or`, `$in`, `$regex`, etc., by expanding the `Predicate` enum.
- **Transactions** – wrap multiple modifications in a higher‑level atomic unit using the WAL.
- **Network Server** – expose the core library via a thin HTTP/WS API to emulate a MongoDB server.

---

*All documentation is placed under the repository’s `docs/` folder to satisfy the global project guidelines.*
