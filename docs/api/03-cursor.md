# Cursor

A `Cursor` is an iterator over the results of a `find` query. Cursors provide a memory-efficient way to process large result sets.

## Creating a Cursor

Cursors are created by calling `find()` on a `Collection`:

```rust
let cursor = users.find(doc! { "age": { "$gte": 18 } })?;
```

## Iterating

### For Loop

```rust
for doc in cursor {
    let doc = doc?;
    println!("{:?}", doc);
}
```

### Collect All

```rust
let docs: Vec<Document> = cursor.collect::<Result<_, _>>()?;
```

## Cursor Options

Cursors can be configured with sorting, skipping, and limiting:

```rust
let cursor = users
    .find(doc! { "status": "active" })
    .sort(doc! { "age": -1 })   // descending
    .skip(10)                    // skip first 10
    .limit(50)                   // return at most 50
    .batch_size(100);            // internal batch size
```

### Sort

```rust
// Ascending
.sort(doc! { "name": 1 })

// Descending
.sort(doc! { "created_at": -1 })

// Compound sort
.sort(doc! { "last_name": 1, "first_name": 1 })
```

| Value | Meaning |
|-------|---------|
| `1` | Ascending order |
| `-1` | Descending order |

### Skip

```rust
// Skip the first N documents
.skip(20)
```

### Limit

```rust
// Return at most N documents
.limit(100)

// Limit of 0 means no limit (return all)
```

### Batch Size

```rust
// Number of documents fetched per internal read
.batch_size(1000)
```

The batch size controls how many documents are fetched in a single storage operation. Larger batches reduce I/O overhead but use more memory.

## Cursor Methods

| Method | Description |
|--------|-------------|
| `next()` | Returns the next document, or `None` if exhausted |
| `sort(spec)` | Set sort order |
| `skip(n)` | Set number of documents to skip |
| `limit(n)` | Set maximum number of documents to return |
| `batch_size(n)` | Set internal fetch batch size |
| `hint(index)` | Force use of a specific index |

## Index Hints

```rust
// Force the query planner to use a specific index
let cursor = users
    .find(doc! { "age": { "$gte": 18 } })
    .hint("age_1")?;
```

## Exhaustion and Reiteration

A cursor is **exhausted** after iterating through all results. Calling `next()` after exhaustion returns `None`. To re-query, create a new cursor:

```rust
// First iteration
for doc in cursor { /* ... */ }

// Cursor is now exhausted — create a new one
let cursor = users.find(doc! { "age": { "$gte": 18 } })?;
```

## Type-Safe Deserialization

Cursors can deserialize documents directly into Rust types:

```rust
#[derive(Debug, Serialize, Deserialize)]
struct User {
    #[serde(rename = "_id")]
    id: ObjectId,
    name: String,
    age: i32,
}

let cursor = users.find(doc! { "age": { "$gte": 18 } })?;
for result in cursor {
    let user: User = result?.deserialize()?;
    println!("{} is {} years old", user.name, user.age);
}
```

## Performance Considerations

- **Batch size** — Increase for sequential scans over large collections; decrease for interactive queries where only a few documents are needed.
- **Skip** — Large skip values are expensive because the cursor must traverse and discard all skipped documents. Consider using range queries on indexed fields instead.
- **Limit** — Always use `limit()` when you only need a subset of results to avoid unnecessary I/O.

## Example: Paginated Query

```rust
fn get_page(collection: &Collection, page: u64, per_page: u64) -> Result<Vec<Document>, Error> {
    let cursor = collection
        .find(doc! {})
        .sort(doc! { "_id": 1 })
        .skip(page * per_page)
        .limit(per_page)
        .batch_size(per_page as usize)?;

    cursor.collect::<Result<Vec<_>, _>>()
}
```
