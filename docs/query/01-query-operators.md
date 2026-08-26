# Query Operators

MongoLite supports a practical subset of MongoDB’s query language. All filters
are BSON documents parsed from JSON using the `serde_json` + `bson` crates.

> Full implementation details live in:
> [`crates/mongolite/src/query.rs`](../../crates/mongolite/src/query.rs)

## Syntax Summary

```rust
let filter = doc! {
    "age":     { "$gt": 25 },
    "$and": [ { "status": "active" }, { "score": { "$gte": 10 } } ]
};
collection.find(Some(filter))?;
```

## Comparison Operators

| Operator | Description              | Example                         |
|----------|--------------------------|---------------------------------|
| `$eq`    | Equal                    | `{ "name": "Alice" }`           |
| `$ne`    | Not equal                | `{ "status": { "$ne": "down" } }` |
| `$gt`    | Greater than             | `{ "age": { "$gt": 25 } }`       |
| `$gte`   | Greater than or equal    | `{ "age": { "$gte": 25 } }`      |
| `$lt`    | Less than                | `{ "age": { "$lt": 65 } }`       |
| `$lte`   | Less than or equal       | `{ "age": { "$lte": 65 } }`      |

Notes:
- Implicit equality (`$eq`) can be used by passing the value directly:
  `{ "name": "Alice" }`.
- Comparisons work across numeric types: int <-> float coercion handled internally.

## Logical Operators

| Operator | Description                              |
|----------|------------------------------------------|
| `$and`   | All sub-conditions must be true          |
| `$or`    | At least one sub-condition must be true  |
| `$nor`   | None of the sub-conditions may be true   |
| `$not`   | Negates a sub-query operator             |

Examples:

```json
{ "$and": [ { "age": { "$gte": 25 } }, { "status": "active" } ] }
{ "$or": [ { "name": "Alice" }, { "name": "Bob" } ] }
{ "age": { "$not": { "$gt": 65 } } }
```

## Element Operators

| Operator | Description                            |
|----------|----------------------------------------|
| `$exists`| True if the field exists in the document |
| `$type`  | Match by BSON type name (see below)    |

Supported `$type` aliases:

| Alias | BSON Type        |
|-------|------------------|
| `double`     | Double        |
| `string`     | String        |
| `bool`       | Boolean       |
| `int`        | Int32         |
| `long`       | Int64         |
| `objectId`   | ObjectId      |
| `array`      | Array         |
| `object`     | Document      |
| `null`       | Null          |
| `date`       | DateTime      |
| `binData`    | Binary        |
| `regex`      | RegularExpression |

## Evaluation Operators

| Operator | Description                                 |
|----------|---------------------------------------------|
| `$regex` | Apply regex pattern against a string value  |

Example:

```json
{ "name": { "$regex": "^Alice.*" } }
```

## Array Operators

| Operator   | Description                                    |
|------------|------------------------------------------------|
| `$in`      | Field value matches any item in provided array |
| `$nin`     | Inverse of `$in`                               |
| `$all`     | Field array contains all elements listed       |
| `$size`    | Match arrays with exact length                 |
| `$elemMatch` | Match at least one element satisfying predicate |

Example:

```json
{ "tags": { "$all": ["rust", "database"] } }
{ "scores": { "$elemMatch": { "$gte": 80 } } }
```

## Nested Field Access

⚠️ **Currently unsupported**: Queries do not descend into nested objects/arrays.
Only top-level keys can be matched directly.

Example of current behavior:

```rust
// ✅ Supported
let filter = doc! { "name": "Alice" };

// ❌ Not supported yet
let filter = doc! { "address.city": "Wonderland" };
```

This is planned for future releases.
