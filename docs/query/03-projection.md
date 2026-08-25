# Projection

Projection controls which fields are included in query results. By default, all fields are returned. Projection allows you to include or exclude specific fields, reducing data transfer and improving performance.

## Basic Projection

### Include Specific Fields

```rust
// Return only "name" and "email" fields (plus "_id")
let cursor = users.find(doc! { "status": "active" })
    .projection(doc! { "name": 1, "email": 1 })?;
```

| Value | Meaning |
|-------|---------|
| `1` | Include this field |
| `0` | Exclude this field |

### Exclude Specific Fields

```rust
// Return all fields except "password" and "internal_notes"
let cursor = users.find(doc! {})
    .projection(doc! { "password": 0, "internal_notes": 0 })?;
```

## Rules

1. **You cannot mix inclusion and exclusion** in the same projection (except for `_id`).
2. **`_id` is always included** unless explicitly excluded.
3. **Inclusion projections** return only the specified fields (plus `_id`).
4. **Exclusion projections** return all fields except the specified ones.

```rust
// VALID: inclusion projection
.projection(doc! { "name": 1, "email": 1 })

// VALID: exclusion projection
.projection(doc! { "password": 0 })

// VALID: exclude _id in inclusion projection
.projection(doc! { "name": 1, "_id": 0 })

// INVALID: mixing inclusion and exclusion (except _id)
.projection(doc! { "name": 1, "password": 0 })  // Error!
```

## Nested Field Projection

### Include Nested Fields

```rust
// Return only "name" and "address.city"
let cursor = users.find(doc! {})
    .projection(doc! {
        "name": 1,
        "address.city": 1,
    })?;
```

### Exclude Nested Fields

```rust
// Return all fields except "address.zip"
let cursor = users.find(doc! {})
    .projection(doc! { "address.zip": 0 })?;
```

## Array Projection

### Slice Operator

```rust
// Return only the first 3 elements of the "tags" array
let cursor = users.find(doc! {})
    .projection(doc! { "tags": { "$slice": 3 } })?;

// Return the last 2 elements
let cursor = users.find(doc! {})
    .projection(doc! { "tags": { "$slice": -2 } })?;

// Return elements 2-4 (skip 2, limit 3)
let cursor = users.find(doc! {})
    .projection(doc! { "tags": { "$slice": [2, 3] } })?;
```

### Positional Operator (`$`)

```rust
// Return only the first matching array element
let cursor = users.find(doc! { "tags": "rust" })
    .projection(doc! { "tags.$": 1 })?;
```

## Projection Examples

### Selective Field Return

```rust
// Get only names and ages of active users
let cursor = users
    .find(doc! { "status": "active" })
    .projection(doc! { "name": 1, "age": 1, "_id": 0 })?;

for doc in cursor {
    let doc = doc?;
    println!("{} — {}", doc.get_str("name")?, doc.get_i32("age")?);
}
```

### Hide Sensitive Fields

```rust
// List all users without exposing passwords
let cursor = users.find(doc! {})
    .projection(doc! {
        "password": 0,
        "ssn": 0,
        "secret_key": 0,
    })?;
```

### Summary Views

```rust
// Get a summary of products (name and price only)
let cursor = products.find(doc! {})
    .projection(doc! {
        "name": 1,
        "price": 1,
        "_id": 0,
    })?;
```

## Performance Considerations

- **Covered queries** — When all projected fields are part of an index, MongoLite can satisfy the query entirely from the index without reading documents.
- **Reduced I/O** — Projecting fewer fields reduces the amount of data read from disk.
- **Network/memory** — Smaller result sets use less memory and are faster to serialize.

```rust
// This query can be covered by an index on { status: 1, name: 1 }
let cursor = users
    .find(doc! { "status": "active" })
    .projection(doc! { "name": 1, "_id": 0 })?;
```

## Projection with Aggregation

In aggregation pipelines, projection is done with the `$project` stage:

```rust
let pipeline = vec![
    doc! { "$match": { "status": "active" } },
    doc! { "$project": {
        "name": 1,
        "age": 1,
        "is_adult": { "$gte": ["$age", 18] },
        "_id": 0,
    }},
];

let results = users.aggregate(pipeline)?;
```

The `$project` stage in aggregation is more powerful than query projection — it can compute new fields, rename fields, and apply expressions.
