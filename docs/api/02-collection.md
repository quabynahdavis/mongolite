# Collection

The `Collection` type represents a group of documents. It provides methods for inserting, querying, updating, and deleting documents.

## Obtaining a Collection

```rust
let users = db.collection("users");
```

Collections are created lazily — no storage is allocated until the first document is inserted.

## Insert Operations

### Insert One

```rust
use bson::doc;

let result = users.insert_one(doc! {
    "name": "Alice",
    "age": 30,
    "email": "alice@example.com",
})?;

println!("Inserted document with _id: {:?}", result.inserted_id);
```

### Insert Many

```rust
let docs = vec![
    doc! { "name": "Bob", "age": 25 },
    doc! { "name": "Carol", "age": 35 },
    doc! { "name": "Dave", "age": 28 },
];

let result = users.insert_many(docs)?;
println!("Inserted {} documents", result.inserted_ids.len());
```

## Query Operations

### Find

```rust
// Find all documents matching a filter
let cursor = users.find(doc! { "age": { "$gte": 25 } })?;

// Find with options
let cursor = users
    .find(doc! { "status": "active" })
    .sort(doc! { "age": -1 })
    .limit(10)
    .skip(20)?;
```

### Find One

```rust
// Find a single document
let user = users.find_one(doc! { "name": "Alice" })?;

// Find one and update
let updated = users.find_one_and_update(
    doc! { "name": "Alice" },
    doc! { "$set": { "last_login": "2026-08-25" } },
)?;
```

### Count

```rust
// Count all documents
let total = users.count_documents(None)?;

// Count matching documents
let active = users.count_documents(Some(doc! { "status": "active" }))?;
```

## Update Operations

### Update One

```rust
let result = users.update_one(
    doc! { "name": "Alice" },
    doc! { "$set": { "age": 31 } },
)?;

println!("Matched: {}, Modified: {}", result.matched_count, result.modified_count);
```

### Update Many

```rust
let result = users.update_many(
    doc! { "status": "pending" },
    doc! { "$set": { "status": "active" } },
)?;

println!("Modified {} documents", result.modified_count);
```

### Upsert

```rust
// Insert if not found, update if found
let result = users.update_one(
    doc! { "name": "Eve" },
    doc! { "$set": { "name": "Eve", "age": 22 } },
    UpdateOptions::new().upsert(true),
)?;
```

## Delete Operations

### Delete One

```rust
let result = users.delete_one(doc! { "name": "Alice" })?;
println!("Deleted {} document(s)", result.deleted_count);
```

### Delete Many

```rust
let result = users.delete_many(doc! { "status": "inactive" })?;
println!("Deleted {} document(s)", result.deleted_count);
```

## Index Operations

### Create Index

```rust
// Create a single-field index
users.create_index(
    doc! { "email": 1 },
    IndexOptions::new().unique(true),
)?;

// Create a compound index
users.create_index(
    doc! { "last_name": 1, "first_name": 1 },
    IndexOptions::new(),
)?;

// Create a text index
users.create_index(
    doc! { "bio": "text" },
    IndexOptions::new(),
)?;
```

### List Indexes

```rust
let indexes = users.list_indexes()?;
for idx in indexes {
    println!("Index: {} — fields: {:?}", idx.name, idx.key);
}
```

### Drop Index

```rust
users.drop_index("email_1")?;
```

## Aggregation

```rust
use mongolite::aggregate;

let pipeline = vec![
    doc! { "$match": { "status": "active" } },
    doc! { "$group": {
        "_id": "$department",
        "count": { "$sum": 1 },
        "avg_age": { "$avg": "$age" },
    }},
    doc! { "$sort": { "count": -1 } },
];

let results = users.aggregate(pipeline)?;
```

## Collection Statistics

```rust
let stats = users.stats()?;
println!("Documents: {}", stats.count);
println!("Data size: {} bytes", stats.size);
println!("Storage size: {} bytes", stats.storage_size);
println!("Indexes: {}", stats.index_count);
```
