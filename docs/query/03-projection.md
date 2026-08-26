# Projection and Sorting

MongoLite’s query engine supports lightweight **projection** and **sorting**
when fetching collections of documents. These features are accessed via
`Collection::find_with_options(...)`.

## Public Interface

```rust
pub fn find_with_options(
    &self,
    filter:     Option<Document>,
    sort:       Option<Document>,
    skip:       Option<u64>,
    limit:      Option<u64>,
    projection: Option<Document>,
) -> Result<Vec<Document>>
```

## Projection

A projection document restricts which fields appear in returned documents.

Rules:
- Mix of include (`{ "field": 1 }`) and exclude (`{ "field": 0 }`) raises
  `Error::InvalidQuery`.
- Omitting `_id` defaults to **including** it.
- Setting `"_id": 0` explicitly excludes it.

### Examples

#### Include Specific Fields

```rust
let projection = doc! { "name": 1, "age": 1 };

let docs = coll.find_with_options(None, None, None, None, Some(projection))?;
// Returns [{"_id": ..., "name": "Alice", "age": 30}]
```

#### Exclude Fields

```rust
let projection = doc! { "password_hash": 0 };

let docs = coll.find_with_options(None, None, None, None, Some(projection))?;
// Password field omitted, everything else included.
```

#### Exclude Both Field and _id

```rust
let projection = doc! { "temp_field": 0, "_id": 0 };

let docs = coll.find_with_options(None, None, None, None, Some(projection))?;
// Drops both temp_field and _id.
```

## Sorting

Sorting applies after filtering. Each key is sorted ascending `(1)` or
descending `(-1)`:

```rust
let sort = doc! { "age": -1, "name": 1 };
let docs = coll.find_with_options(None, Some(sort), None, None, None)?;
```

Comparison rules:
- Integer/float are coerced for mixed-type comparisons.
- Strings compared lexically.
- Null is treated as smallest element.
- Missing fields sort as nulls.

## Skip and Limit

```rust
// Skip first 10 docs, return next 5
let docs = coll.find_with_options(
    None, None,
    Some(10), Some(5),
    None
)?;
```

## Summary Table

| Option       | Argument      | Effect                           |
|--------------|---------------|----------------------------------|
| `filter`     | `Option<Document>` | Restrict matching documents |
| `sort`       | `Option<Document>` | Order results                |
| `skip`       | `Option<u64>`   | Number of initial docs skipped |
| `limit`      | `Option<u64>`   | Cap total docs returned        |
| `projection` | `Option<Document>` | Choose visible fields         |

---

See also: [Query Operators](01-query-operators.md)
