# Database

The `Database` type represents a single MongoLite database backed by a `.mongolite` file. It is the entry point for all database operations.

## Opening a Database

```rust
use mongolite::Database;

// Create or open a database file
let db = Database::open("myapp.mongolite")?;
```

If the file does not exist, it will be created with default settings. If it exists, the database will be opened and its state restored (including WAL recovery if needed).

### Options

```rust
use mongolite::DatabaseOptions;

let opts = DatabaseOptions::new()
    .page_size(8192)
    .cache_size_mb(64)
    .create_if_missing(true);

let db = Database::open_with_options("myapp.mongolite", opts)?;
```

| Option | Default | Description |
|--------|---------|-------------|
| `page_size` | 4096 | Page size in bytes (4096, 8192, 16384, or 32768) |
| `cache_size_mb` | 32 | Maximum page cache size in megabytes |
| `create_if_missing` | `true` | Create the file if it does not exist |
| `read_only` | `false` | Open in read-only mode |

## Getting a Collection

```rust
// Get or create a collection by name
let users = db.collection("users");
let products = db.collection("products");
```

Calling `collection()` does not allocate any resources until an operation is performed. Collections are created lazily on first write.

## Listing Collections

```rust
// List all user-created collections
let names = db.list_collections()?;
for name in names {
    println!("Collection: {}", name);
}
```

## Dropping a Collection

```rust
// Drop a collection and all its data
db.drop_collection("users")?;
```

This removes all documents, indexes, and metadata associated with the collection.

## Database Statistics

```rust
// Get database-level statistics
let stats = db.stats()?;
println!("Collections: {}", stats.collection_count);
println!("Documents: {}", stats.document_count);
println!("Data size: {} bytes", stats.data_size);
println!("File size: {} bytes", stats.file_size);
```

## Closing a Database

```rust
// Explicitly close the database (flushes all pending writes)
db.close()?;
```

The database is also automatically closed when it is dropped (via the `Drop` trait), but explicit closing allows you to handle errors.

## Read-Only Mode

```rust
// Open an existing database in read-only mode
let opts = DatabaseOptions::new()
    .read_only(true);

let db = Database::open_with_options("myapp.mongolite", opts)?;
```

In read-only mode, any write operation will return an `InvalidArgument` error.

## Example: Full Workflow

```rust
use mongolite::Database;
use bson::doc;

fn main() -> Result<(), mongolite::Error> {
    let db = Database::open("example.mongolite")?;

    // Create collections
    let users = db.collection("users");
    let orders = db.collection("orders");

    // Insert sample data
    users.insert_one(doc! {
        "name": "Alice",
        "email": "alice@example.com",
    })?;

    // Check what collections exist
    let names = db.list_collections()?;
    assert!(names.contains(&"users".to_string()));
    assert!(names.contains(&"orders".to_string()));

    // Clean up
    db.drop_collection("orders")?;

    db.close()?;
    Ok(())
}
```
