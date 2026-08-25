# Query Documentation

This folder covers MongoLite's query language, which is compatible with MongoDB's query and update operators. It describes how to filter documents, apply updates, and shape query results.

## Contents

| File | Description |
|------|-------------|
| [01-query-operators](./01-query-operators.md) | Query operators: comparison, logical, element, array, and evaluation operators |
| [02-update-operators](./02-update-operators.md) | Update operators: field set, increment, array manipulation, and more |
| [03-projection](./03-projection.md) | Projection operators: controlling which fields are returned |

## Query Language Overview

MongoLite uses MongoDB's declarative query language. Queries are expressed as BSON documents:

```rust
// Find all users aged 25 or older
let cursor = users.find(doc! { "age": { "$gte": 25 } })?;

// Find users in specific cities
let cursor = users.find(doc! {
    "city": { "$in": ["NYC", "LA", "Chicago"] }
})?;

// Compound query
let cursor = users.find(doc! {
    "$and": [
        { "age": { "$gte": 18 } },
        { "status": "active" }
    ]
})?;
```

## Operator Categories

| Category | Examples | Purpose |
|----------|----------|---------|
| Comparison | `$eq`, `$ne`, `$gt`, `$gte`, `$lt`, `$lte`, `$in`, `$nin` | Compare values |
| Logical | `$and`, `$or`, `$nor`, `$not` | Combine conditions |
| Element | `$exists`, `$type` | Check field existence/type |
| Evaluation | `$regex`, `$text`, `$expr` | Advanced matching |
| Array | `$all`, `$elemMatch`, `$size` | Array-specific queries |

## Reading Order

1. Start with [01-query-operators.md](./01-query-operators.md) to learn how to filter documents.
2. Continue with [02-update-operators.md](./02-update-operators.md) to learn how to modify documents.
3. Finish with [03-projection.md](./03-projection.md) to learn how to control output shape.
