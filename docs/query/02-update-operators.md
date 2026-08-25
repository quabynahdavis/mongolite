# Update Operators

MongoLite supports MongoDB's update operators for modifying documents. These operators allow precise, atomic changes without replacing the entire document.

## Field Update Operators

### `$set` — Set Field Value

```rust
// Set a single field
users.update_one(
    doc! { "_id": 1 },
    doc! { "$set": { "status": "active" } },
)?;

// Set multiple fields
users.update_one(
    doc! { "_id": 1 },
    doc! { "$set": {
        "name": "Alice",
        "email": "alice@example.com",
        "updated_at": "2026-08-25",
    }},
)?;

// Set a nested field
users.update_one(
    doc! { "_id": 1 },
    doc! { "$set": { "address.city": "Springfield" } },
)?;
```

### `$unset` — Remove Field

```rust
// Remove a field entirely
users.update_one(
    doc! { "_id": 1 },
    doc! { "$unset": { "temporary_field": "" } },
)?;
```

> **Note:** The value specified in `$unset` is ignored (conventionally set to `""`).

### `$rename` — Rename Field

```rust
// Rename a field
users.update_one(
    doc! { "_id": 1 },
    doc! { "$rename": { "old_name": "new_name" } },
)?;
```

### `$setOnInsert` — Set on Insert (Upsert)

```rust
// Only set the field when inserting a new document (upsert)
users.update_one(
    doc! { "_id": 1 },
    doc! {
        "$set": { "last_login": "2026-08-25" },
        "$setOnInsert": { "created_at": "2026-08-25" },
    },
    UpdateOptions::new().upsert(true),
)?;
```

## Numeric Update Operators

### `$inc` — Increment

```rust
// Increment a field by 1
users.update_one(
    doc! { "_id": 1 },
    doc! { "$inc": { "login_count": 1 } },
)?;

// Decrement (negative increment)
products.update_one(
    doc! { "_id": 1 },
    doc! { "$inc": { "stock": -5 } },
)?;
```

### `$mul` — Multiply

```rust
// Multiply a field by a factor
products.update_one(
    doc! { "_id": 1 },
    doc! { "$mul": { "price": 1.1 } },  // 10% price increase
)?;
```

### `$min` — Minimum

```rust
// Only update if the new value is lower
products.update_one(
    doc! { "_id": 1 },
    doc! { "$min": { "lowest_price": 15.99 } },
)?;
```

### `$max` — Maximum

```rust
// Only update if the new value is higher
products.update_one(
    doc! { "_id": 1 },
    doc! { "$max": { "highest_price": 29.99 } },
)?;
```

## Array Update Operators

### `$push` — Append to Array

```rust
// Append a single element
users.update_one(
    doc! { "_id": 1 },
    doc! { "$push": { "tags": "rust" } },
)?;

// Append multiple elements
users.update_one(
    doc! { "_id": 1 },
    doc! { "$push": { "tags": { "$each": ["rust", "database"] } } },
)?;
```

### `$push` with Modifiers

```rust
// Append and keep only the first 5 elements
users.update_one(
    doc! { "_id": 1 },
    doc! { "$push": {
        "recent_logins": {
            "$each": ["2026-08-25"],
            "$slice": 5,
        }
    }},
)?;

// Append and sort
users.update_one(
    doc! { "_id": 1 },
    doc! { "$push": {
        "scores": {
            "$each": [85, 92, 78],
            "$sort": -1,
        }
    }},
)?;

// Append at a specific position
users.update_one(
    doc! { "_id": 1 },
    doc! { "$push": {
        "tags": {
            "$each": ["new_tag"],
            "$position": 0,  // Insert at beginning
        }
    }},
)?;
```

### `$pop` — Remove from Array

```rust
// Remove the last element
users.update_one(
    doc! { "_id": 1 },
    doc! { "$pop": { "tags": 1 } },
)?;

// Remove the first element
users.update_one(
    doc! { "_id": 1 },
    doc! { "$pop": { "tags": -1 } },
)?;
```

### `$pull` — Remove by Condition

```rust
// Remove all occurrences of a value
users.update_one(
    doc! { "_id": 1 },
    doc! { "$pull": { "tags": "deprecated" } },
)?;

// Remove elements matching a condition
users.update_one(
    doc! { "_id": 1 },
    doc! { "$pull": { "scores": { "$lt": 50 } } },
)?;
```

### `$pullAll` — Remove Multiple Values

```rust
// Remove all specified values
users.update_one(
    doc! { "_id": 1 },
    doc! { "$pullAll": { "tags": ["old", "deprecated", "test"] } },
)?;
```

### `$addToSet` — Add to Set (Unique)

```rust
// Add only if not already present
users.update_one(
    doc! { "_id": 1 },
    doc! { "$addToSet": { "tags": "rust" } },
)?;

// Add multiple unique values
users.update_one(
    doc! { "_id": 1 },
    doc! { "$addToSet": {
        "tags": { "$each": ["rust", "database", "new_tag"] }
    }},
)?;
```

## Bitwise Update Operators

### `$bit` — Bitwise Operation

```rust
// Bitwise AND
users.update_one(
    doc! { "_id": 1 },
    doc! { "$bit": { "permissions": { "and": 0x0004 } } },
)?;

// Bitwise OR
users.update_one(
    doc! { "_id": 1 },
    doc! { "$bit": { "permissions": { "or": 0x0004 } } },
)?;

// Bitwise XOR
users.update_one(
    doc! { "_id": 1 },
    doc! { "$bit": { "permissions": { "xor": 0x0004 } } },
)?;
```

## Combining Update Operators

Multiple update operators can be used in a single update operation:

```rust
users.update_one(
    doc! { "_id": 1 },
    doc! {
        "$set": { "status": "active" },
        "$inc": { "login_count": 1 },
        "$push": {
            "recent_logins": {
                "$each": ["2026-08-25"],
                "$slice": 10,
            }
        },
        "$pull": { "tags": "inactive" },
    },
)?;
```

## Upsert

When `upsert: true` is specified, MongoLite inserts a new document if no match is found:

```rust
let result = users.update_one(
    doc! { "email": "newuser@example.com" },
    doc! {
        "$set": {
            "name": "New User",
            "email": "newuser@example.com",
        },
        "$setOnInsert": {
            "created_at": "2026-08-25",
        },
    },
    UpdateOptions::new().upsert(true),
)?;

if result.upserted_id.is_some() {
    println!("New user created");
}
```

## Update vs Replace

| Operation | Behavior |
|-----------|----------|
| `update_one` with operators | Modifies specific fields; preserves other fields |
| `replace_one` | Replaces the entire document (except `_id`) |

```rust
// Update: only changes the "status" field
users.update_one(
    doc! { "_id": 1 },
    doc! { "$set": { "status": "active" } },
)?;

// Replace: replaces the entire document
users.replace_one(
    doc! { "_id": 1 },
    doc! { "name": "Alice", "status": "active" },  // All other fields are lost
)?;
```
