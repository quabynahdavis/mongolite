# Query Operators

MongoLite supports MongoDB's query operators for filtering documents. This document covers all operator categories with examples.

## Comparison Operators

Comparison operators compare a field value to a specified value.

### `$eq` — Equal

```rust
// Find documents where age equals 25
users.find(doc! { "age": { "$eq": 25 } })?;

// Shorthand (implicit $eq)
users.find(doc! { "age": 25 })?;
```

### `$ne` — Not Equal

```rust
// Find documents where status is not "inactive"
users.find(doc! { "status": { "$ne": "inactive" } })?;
```

### `$gt` — Greater Than

```rust
// Find documents where age is greater than 25
users.find(doc! { "age": { "$gt": 25 } })?;
```

### `$gte` — Greater Than or Equal

```rust
// Find documents where age is 25 or older
users.find(doc! { "age": { "$gte": 25 } })?;
```

### `$lt` — Less Than

```rust
// Find documents where price is less than 100
products.find(doc! { "price": { "$lt": 100 } })?;
```

### `$lte` — Less Than or Equal

```rust
// Find documents where price is 100 or less
products.find(doc! { "price": { "$lte": 100 } })?;
```

### `$in` — In Array

```rust
// Find documents where status is one of the listed values
users.find(doc! { "status": { "$in": ["active", "pending"] } })?;

// Find documents where city is one of the listed values
users.find(doc! { "city": { "$in": ["NYC", "LA", "Chicago"] } })?;
```

### `$nin` — Not In Array

```rust
// Find documents where status is none of the listed values
users.find(doc! { "status": { "$nin": ["banned", "deleted"] } })?;
```

## Logical Operators

Logical operators combine multiple conditions.

### `$and` — Logical AND

```rust
// Find documents matching ALL conditions
users.find(doc! {
    "$and": [
        { "age": { "$gte": 18 } },
        { "age": { "$lte": 65 } },
        { "status": "active" }
    ]
})?;

// Shorthand (implicit $and)
users.find(doc! {
    "age": { "$gte": 18, "$lte": 65 },
    "status": "active"
})?;
```

### `$or` — Logical OR

```rust
// Find documents matching ANY condition
users.find(doc! {
    "$or": [
        { "status": "admin" },
        { "role": "moderator" }
    ]
})?;
```

### `$nor` — Logical NOR

```rust
// Find documents matching NONE of the conditions
users.find(doc! {
    "$nor": [
        { "status": "banned" },
        { "status": "deleted" }
    ]
})?;
```

### `$not` — Logical NOT

```rust
// Find documents where age is NOT greater than 25
users.find(doc! { "age": { "$not": { "$gt": 25 } } })?;
```

## Element Operators

Element operators check the existence or type of fields.

### `$exists` — Field Existence

```rust
// Find documents that have an "email" field
users.find(doc! { "email": { "$exists": true } })?;

// Find documents that do NOT have a "phone" field
users.find(doc! { "phone": { "$exists": false } })?;
```

### `$type` — Field Type

```rust
// Find documents where "age" is a number
users.find(doc! { "age": { "$type": "number" } })?;

// Find documents where "tags" is an array
users.find(doc! { "tags": { "$type": "array" } })?;

// Find documents where "name" is a string
users.find(doc! { "name": { "$type": "string" } })?;
```

| Type String | BSON Type |
|-------------|-----------|
| `"double"` | Double |
| `"string"` | String |
| `"object"` | Document |
| `"array"` | Array |
| `"binData"` | Binary |
| `"objectId"` | ObjectId |
| `"bool"` | Boolean |
| `"date"` | UTC datetime |
| `"null"` | Null |
| `"int"` | Int32 |
| `"long"` | Int64 |
| `"number"` | Any numeric type |

## Array Operators

Array operators provide specialized matching for array fields.

### `$all` — Contains All

```rust
// Find documents where tags contains ALL specified values
users.find(doc! { "tags": { "$all": ["rust", "database"] } })?;
```

### `$elemMatch` — Element Match

```rust
// Find documents where the "scores" array has an element
// that is both >= 80 and <= 90
users.find(doc! {
    "scores": {
        "$elemMatch": {
            "$gte": 80,
            "$lte": 90
        }
    }
})?;
```

### `$size` — Array Size

```rust
// Find documents where "tags" has exactly 3 elements
users.find(doc! { "tags": { "$size": 3 } })?;
```

## Evaluation Operators

Evaluation operators provide advanced matching capabilities.

### `$regex` — Regular Expression

```rust
// Find documents where name starts with "Al"
users.find(doc! { "name": { "$regex": "^Al" } })?;

// Case-insensitive search
users.find(doc! { "name": { "$regex": "alice", "$options": "i" } })?;
```

| Option | Description |
|--------|-------------|
| `i` | Case-insensitive |
| `m` | Multiline mode |
| `s` | Dot matches newline |
| `x` | Extended (ignore whitespace) |

### `$text` — Text Search

```rust
// Full-text search (requires a text index)
users.find(doc! { "$text": { "$search": "rust database" } })?;

// Search with language
users.find(doc! {
    "$text": {
        "$search": "rust",
        "$language": "english"
    }
})?;
```

### `$expr` — Expression

```rust
// Find documents where the value of "total" equals "price" + "tax"
orders.find(doc! {
    "$expr": {
        "$eq": ["$total", { "$add": ["$price", "$tax"] }]
    }
})?;
```

## Dot Notation

Access nested fields using dot notation:

```rust
// Match documents where address.city is "Springfield"
users.find(doc! { "address.city": "Springfield" })?;

// Match nested array elements
users.find(doc! { "comments.0.approved": true })?;
```

## Operator Precedence

When multiple operators are present in a query:

1. **Implicit `$and`** — All top-level conditions are ANDed together.
2. **`$not`** — Applied to a single field's condition.
3. **`$and` / `$or` / `$nor`** — Explicit logical grouping.

```rust
// This query:
doc! {
    "age": { "$gte": 18, "$lte": 65 },
    "$or": [
        { "status": "active" },
        { "role": "admin" }
    ]
}

// Is equivalent to:
doc! {
    "$and": [
        { "age": { "$gte": 18 } },
        { "age": { "$lte": 65 } },
        {
            "$or": [
                { "status": "active" },
                { "role": "admin" }
            ]
        }
    ]
}
```
