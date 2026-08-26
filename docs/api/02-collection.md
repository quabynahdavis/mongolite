# Collection API

The `Collection` struct is instantiated via `Database::collection()` and provides
all interactions with a logical grouping of documents.

## Importing

```rust
use mongolite::Database;
let mut db = Database::create("db.mongolite")?;
let mut users = db.collection("users");
```

## Methods

### `name() -> &str`

Returns the name of the collection.

```rust
println!("Working with collection: {}", users.name()); // "users"
```

### `insert_one(doc: Document) -> Result<InsertOneResult>`

Inserts a single document. Auto-generates an `_id` if absent.

```rust
use bson::doc;

let result = users.insert_one(doc! {
    "name": "Alice",
    "age": 30
})?;

println!("Inserted _id: {}", result.inserted_id);
```

#### InsertOneResult

```rust
pub struct InsertOneResult {
    pub inserted_id: ObjectId,
}
```

### `insert_many(docs: Vec<Document>) -> Result<InsertManyResult>`

Bulk insert multiple documents.

```rust
let docs = vec![
    doc! { "name": "Alice" },
    doc! { "name": "Bob" },
];
let result = users.insert_many(docs)?;
assert_eq!(result.inserted_ids.len(), 2);
```

#### InsertManyResult

```rust
pub struct InsertManyResult {
    pub inserted_ids: Vec<ObjectId>,
}
```

### `find(filter: Option<Document>) -> Result<Vec<Document>>`

Performs a linear scan over all documents, applying the filter predicate.

```rust
let docs = users.find(doc! { "age": { "$gt": 25 } })?;
assert!(!docs.is_empty());
```

### `find_one(filter: Option<Document>) -> Result<Option<Document>>`

Short-circuits at the first match.

```rust
let maybe_doc = users.find_one(doc! { "name": "Alice" })?;
assert!(maybe_doc.is_some());
```

### `find_with_options(...)`

Advanced finder supporting sorting, skipping, limiting, and projection.

```rust
let docs = users.find_with_options(
    Some(doc! { "age": { "$gt": 20 } }),
    Some(doc! { "age": -1 }), // sort descending
    Some(0),                   // skip none
    Some(10),                  // limit to 10 docs
    Some(doc! { "name": 1 })    // project only name
)?;
```

### `count(filter: Option<Document>) -> Result<u64>`

Counts matching documents.

```rust
let total = users.count(None)?;                     // all docs
let adults = users.count(doc! { "age": { "$gte": 18 } })?;
```

### `update_one(filter, update) -> Result<UpdateResult>`

Applies `$set` / `$unset` operations to one matching document.

```rust
let updated = users.update_one(
    doc! { "_id": alice_id },
    doc! { "$set": { "age": 31 } }
)?;

assert_eq!(updated.matched_count, 1);
assert_eq!(updated.modified_count, 1);
```

#### UpdateResult

```rust
pub struct UpdateResult {
    pub matched_count: u64,
    pub modified_count: u64,
}
```

### `update_many(...)`

Same semantics as `update_one` but applies to **all** matching documents.

### `delete_one(filter) -> Result<DeleteResult>`

Deletes the first document matching `filter`.

```rust
let result = users.delete_one(
    doc! { "_id": alice_id }
)?;
assert_eq!(result.deleted_count, 1);
```

#### DeleteResult

```rust
pub struct DeleteResult {
    pub deleted_count: u64,
}
```

### `delete_many(...)`

Same semantics as `delete_one` but removes all matching documents.

### `drop() -> Result<bool>`

Removes the entire collection.

## Filter Constraints

⚠️ **Important:** Update and delete methods currently require a `_id` field in
the filter document. Passing a filter without `_id` will return an
`Error::InvalidQuery`.

## Error Handling

All methods return `Result<T, Error>`. Common errors:

| Error Type             | Triggered When                              |
|------------------------|---------------------------------------------|
| `InvalidQuery`         | Missing `_id`, unsupported regex, unknown ops |
| `Corrupted`            | Underlying page checksum mismatch           |
| `Bson`                 | Serialization failure                       |

## Next Steps

- See [Database API](01-database.md) for opening/closing databases.
- Review [Cursor API](03-cursor.md) for iteration helpers.
- For querying rules, consult [Query Operators](../query/01-query-operators.md).
