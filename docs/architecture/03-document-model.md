# Document Model

MongoLite uses the BSON (Binary JSON) document format, ensuring compatibility with MongoDB's type system and query language.

## Overview

A **document** is an ordered set of key-value pairs. Documents are the fundamental unit of data in MongoLite, equivalent to a row in a relational database but with a flexible, nested structure.

```rust
// Example document
{
    "_id": ObjectId("507f1f77bcf86cd799439011"),
    "name": "Alice",
    "age": 30,
    "address": {
        "street": "123 Main St",
        "city": "Springfield"
    },
    "tags": ["developer", "rust"]
}
```

## BSON Type System

MongoLite supports the full BSON type specification. Each type has a type byte identifier used in the binary encoding.

| Type | ID | Description | Rust Mapping |
|------|----|-------------|--------------|
| `Double` | `0x01` | 64-bit IEEE 754 float | `f64` |
| `String` | `0x02` | UTF-8 string | `String` |
| `Document` | `0x03` | Embedded document | `Document` |
| `Array` | `0x04` | Array of values | `Vec<Bson>` |
| `Binary` | `0x05` | Binary data | `Vec<u8>` |
| `ObjectId` | `0x07` | 12-byte ObjectId | `ObjectId` |
| `Boolean` | `0x08` | Boolean | `bool` |
| `UTC datetime` | `0x09` | 64-bit timestamp (ms) | `DateTime<Utc>` |
| `Null` | `0x0A` | Null value | `Bson::Null` |
| `Regex` | `0x0B` | Regular expression | `Regex` |
| `Int32` | `0x10` | 32-bit signed integer | `i32` |
| `Int64` | `0x12` | 64-bit signed integer | `i64` |
| `Decimal128` | `0x13` | 128-bit decimal | `Decimal128` |

## ObjectId

Every document must have an `_id` field that uniquely identifies it within a collection. If you do not provide one, MongoLite automatically generates an `ObjectId`.

An ObjectId is a 12-byte value:

```
┌─────────────┬──────────┬────────┬──────────┐
│ Timestamp   │ Machine  │ PID    │ Counter  │
│ (4 bytes)   │ (3 bytes)│(2 bytes)│(3 bytes) │
└─────────────┴──────────┴────────┴──────────┘
```

- **Timestamp** — Seconds since Unix epoch.
- **Machine** — Machine identifier (hash of hostname).
- **PID** — Process ID.
- **Counter** — Monotonically increasing counter (random initial value).

## Document Size Limits

- **Maximum document size:** 16 MiB (16,777,216 bytes) after BSON encoding.
- **Maximum nesting depth:** 100 levels.
- **Maximum field name length:** No hard limit, but field names should be kept reasonable (recommended < 256 bytes).

## Field Names

- Field names are UTF-8 strings.
- The `$` prefix is reserved for operators (used in queries).
- The `.` character is used for dot notation in nested field access.
- Field names cannot contain null bytes.

## Collections

A **collection** is a group of documents. Collections are schema-less — documents within the same collection can have different structures.

```
Database
├── users (collection)
│   ├── { _id: 1, name: "Alice", age: 30 }
│   ├── { _id: 2, name: "Bob", email: "bob@example.com" }
│   └── { _id: 3, name: "Carol", tags: ["admin"] }
├── products (collection)
│   ├── { _id: 1, name: "Widget", price: 9.99 }
│   └── { _id: 2, name: "Gadget", price: 19.99 }
└── _indexes (system collection)
    └── { name: "users_age_idx", key: { age: 1 } }
```

## Type Comparison Order

When comparing values of different types (e.g., in sorting), MongoLite follows MongoDB's comparison order:

1. MinKey (internal type)
2. Null
3. Numbers (ints, longs, doubles, decimals)
4. Symbol, String
5. Object (Document)
6. Array
7. BinData
8. ObjectId
9. Boolean
10. Date
11. Timestamp
12. Regular Expression
13. MaxKey (internal type)

## Rust API Mapping

In Rust, documents are represented using the `Document` type from the `bson` crate:

```rust
use bson::{doc, Document};

// Create a document
let doc = doc! {
    "name": "Alice",
    "age": 30,
    "active": true,
};

// Access fields
let name = doc.get_str("name")?;
let age = doc.get_i32("age")?;

// Nested documents
let doc = doc! {
    "address": {
        "city": "Springfield",
        "zip": "62704"
    }
};
let city = doc.get_document("address")?.get_str("city")?;
```
