# Update Operators

MongoLite supports a minimal set of update modifiers for modifying existing documents.

> Full implementation lives in:
> [`crates/mongolite/src/collection.rs`](../../crates/mongolite/src/collection.rs)

## Supported Operators

| Operator | Description                           | Behavior                          |
|----------|---------------------------------------|-----------------------------------|
| `$set`   | Sets the value of a field             | Creates the field if missing      |
| `$unset` | Removes a field                       | Fails gracefully if field absent  |

Any update key not starting with `$` is treated as an implicit `$set`.

## Usage Patterns

### Basic `$set`

```rust
let filter  = doc! { "_id": some_object_id };
let update  = doc! { "$set": { "age": 31 } };

coll.update_one(filter, update)?;
```

### Multiple Fields

```rust
let update = doc! {
    "$set": {
        "name": "Alice Smith",
        "updated": bson::DateTime::now().to_chrono(),
    }
};

coll.update_one(filter, update)?;
```

### Remove a Field (`$unset`)

```rust
let update = doc! {
    "$unset": { "temp_flag": "" }
};

coll.update_one(filter, update)?;
```

> 📝 Note: The value associated with `$unset` is ignored — presence of the key
> triggers removal regardless of its right-hand side.

### Mixed Update

Mixing `$set` and `$unset` in a single call is allowed:

```rust
let update = doc! {
    "$set": { "last_login": bson::DateTime::now().to_chrono() },
    "$unset": { "session_token": "" }
};

coll.update_one(filter, update)?;
```

## Filter Requirements

⚠️ **Current Limitation**

- Updates require a filter containing **`_id`** and will refuse filters that don't have it.
- Multi-document atomic updates are not supported.
- No `$inc`, `$push`, `$pull`, or other advanced operators exist yet.

Future enhancements might add:
- `$inc` for incrementing numbers.
- `$push` / `$pop` for array manipulation.
- Nested path updates (`"address.city"`).
