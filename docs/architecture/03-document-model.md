# Document Model

MongoLite stores all data as **BSON documents**. Each document is a
sequence of named fields (key-value pairs).

## Field Value Types

The `bson` crate (version 2.x) handles serialization; MongoLite adds a
thin wrapper (`mongolite::document::Document`) plus a custom `ObjectId`.

| BSON Type      | Native Rust Equivalent       | Notes                       |
|----------------|------------------------------|-----------------------------|
| `String`       | `String`                     | UTF-8 encoded               |
| `Int32`        | `i32`                        | 32-bit signed integer       |
| `Int64`        | `i64`                        | 64-bit signed integer       |
| `Double`       | `f64`                        | IEEE 754 double             |
| `Boolean`      | `bool`                       |                             |
| `Null`         | `()`                         | Often used to mean "missing" |
| `ObjectId`     | `mongolite::document::ObjectId` | 12-byte unique identifier |
| `Array`        | `Vec<Bson>`                  | Ordered heterogeneous list  |
| `Document`     | `bson::Document`             | Nested key/value map        |
| `DateTime`     | `chrono::DateTime` or `i64`  | UTC timestamp               |
| `Binary`       | `Vec<u8>`                    | Generic blob                |

## ObjectId Format

MongoLite's `ObjectId` differs slightly from the canonical 12-byte ObjectId
used by upstream `bson` in that it uses a **constant machine ID** derived from
a compile-time hash of `"mongolite"` rather than a random machine fingerprint.
This ensures deterministic uniqueness across processes but avoids leaking
machine entropy into keys.

Layout of the 12 bytes:

```
[0..3]   Timestamp in seconds since the Unix epoch (big-endian).
[4..9]   Machine identifier (5 bytes, constant derived from "mongolite").
[9..12]  Counter value. Increments once per `ObjectId::new()` call.
          Stored in **big-endian**, with only the lower 3 bytes used
          (top byte discarded).
```

This allows efficient ordering by creation time while guaranteeing near-zero
collision probability under reasonable usage constraints.

## Document Lifecycle

1. A Rust `bson::Document` is converted to internal `mongolite::document::Document`.
2. On insert, if no `_id` field exists, one is generated and inserted.
3. The document is serialized into BSON binary form using `bson::to_vec`.
4. Key = `_id` value (12 bytes), Value = BSON bytes.
5. These `(key, value)` pairs are inserted into the collection’s user-facing
   B+Tree index.

## Nested Documents and Arrays

Because BSON supports nesting natively, MongoLite supports nested paths:

```json
{
  "name": "Alice",
  "address": {
    "city": "Wonderland",
    "zip": "12345"
  },
  "tags": ["engineer", "inventor"]
}
```

However, there are currently **no dot-notation queries** supported outside
root-level keys. Nested field access requires custom filtering logic until
such features land in future releases.

## Projections and Updates

MongoLite supports field projection via `find_with_options(projection=...)`,
which copies only the requested keys out of the matched documents. Update
operations (`$set`, `$unset`) affect top-level fields only.

For details on query/update operators, refer to:
- [docs/query/01-query-operators.md](../query/01-query-operators.md)
- [docs/query/02-update-operators.md](../query/02-update-operators.md)
- [docs/query/03-projection.md](../query/03-projection.md)
