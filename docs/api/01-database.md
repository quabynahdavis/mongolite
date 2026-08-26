# Database API

The `Database` struct provides the entry point to working with MongoLite.

## Importing

```rust
use mongolite::Database;
```

## Construction

### `create<P: AsRef<Path>>(path: P) -> Result<Database>`

Creates a new database file at the given path. Fails if the file already exists.

```rust
let mut db = Database::create("mydb.mongolite")?;
```

### `open<P: AsRef<Path>>(path: P) -> Result<Database>`

Opens an existing database file.

```rust
let mut db = Database::open("existing.mongolite")?;
```

## Methods

### `collection(name: &str) -> Collection`

Returns a handle to a named collection. The collection is auto-created on first
insert — no explicit creation needed.

```rust
let mut users = db.collection("users");
users.insert_one(doc! { "name": "Alice" })?;
```

### `list_collections() -> Result<Vec<String>>`

Lists all collection names stored in the catalog.

```rust
for coll_name in db.list_collections()? {
    println!("Collection: {}", coll_name);
}
```

### `drop_collection(name: &str) -> Result<bool>`

Drops a named collection from the database.

Returns `true` if the collection was found and deleted, `false` otherwise.

```rust
if db.drop_collection("temp_data")? {
    println!("Dropped temp_data collection");
}
```

### `flush(&mut self) -> Result<()>`

Forces pending changes to be flushed to disk.

```rust
db.flush()?; // Ensure durability after writes
```

## Internal Accessors (Used Mostly by Tests)

These methods are hidden behind `pub(crate)` visibility:

- `allocator_mut(&mut self) -> &mut Allocator`
- `catalog_mut(&mut self) -> &mut BTree`

They are exposed here for completeness, but should not be called outside the crate.

## Error Variants

See [docs/CHANGELOG.md](../../CHANGELOG.md) for version history. For runtime
error definitions, refer to the [Error enum API documentation](02-collection.md).
