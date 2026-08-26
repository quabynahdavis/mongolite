# MongoLite API Reference

> The authoritative source of truth lives in the [Rust source](...) itself.
> This document is a guide to the public API surface surfaced by the
> `mongolite` crate (re-exported by `mongolite::db::Database`).

---

## 1. Entry Point – `Database`

```rust
pub fn create<P: AsRef<Path>>(path: P) -> Result<Database>
pub fn open  <P: AsRef<Path>>(path: P) -> Result<Database>
```

| Action | Notes |
|--------|-------|
| `create` | Creates a new `.mongoLite` file at `path`. File is zero-initialized except for the header. |
| `open` | Opens an existing file; runs checksum validation and optional WAL replay. |
| `flush` | Forces dirty pages to disk. |
| `collection(name)` | Returns a handle to a named collection. If the collection does not yet exist, it is *created on first insert*. No explicit `create_collection` needed. |
| `list_collections` | Scans the catalog B-Tree and returns `Vec<String>`. |
| `drop_collection(name)` | Removes the collection’s B-Tree root from the catalog. |

---

## 2. Collection – CRUD Primer

```rust
let mut users = db.collection("users");
let inserted = users.insert_one(doc! { "name": "Alice", "age": 30 })?;
let found    = users.find(doc! { "age": { "$gt": 25 } })?;
let count    = users.count(doc! { "age": { "$gte": 18 } })?;
```

### 2.1 Insert

```rust
pub fn insert_one(&mut self, doc: Document) -> Result<InsertOneResult>
pub fn insert_many(&mut self, docs: Vec<Document>) -> Result<InsertManyResult>
```

- If the document lacks `_id`, a new `ObjectId` is generated.
- Result contains the assigned `inserted_id`.

### 2.2 Read

```rust
pub fn find      (&self, filter: Option<Document>) -> Result<Vec<Document>>
pub fn find_one  (&self, filter: Option<Document>) -> Result<Option<Document>>
pub fn count     (&self, filter: Option<Document>) -> Result<u64>
```

Under the hood, `find` performs a full B-Tree scan and applies the
`QueryMatcher` predicate to every entry. This is **O(n)** today — there
are no secondary indexes in this version, only the implicit primary-key
index on `_id`.

### 2.3 Update

```rust
pub fn update_one(&mut self, filter: Document, update: Document) -> Result<UpdateResult>
pub fn update_many(&mut self, filter: Document, update: Document) -> Result<UpdateResult>
```

Supported update operators (see [docs/query/02-update-operators.md](query/02-update-operators.md)):

| Operator | Description |
|----------|-------------|
| `$set`   | Overwrite the value at `key`. |
| `$unset` | Remove the value at `key`. |

Update filters currently **must contain an `_id`**; partial matches are
not yet supported outside of `_id`-based operations.

### 2.4 Delete

```rust
pub fn delete_one(&mut self, filter: Document) -> Result<DeleteResult>
pub fn delete_many(&mut self, filter: Document) -> Result<DeleteResult>
```

Same `_id`-restriction as updates.

---

## 3. Query Operators

### Comparison
- `$eq`, `$ne`, `$gt`, `$gte`, `$lt`, `$lte`

### Logical
- `$and` *(top-level only)*
- `$or` *(top-level only)*
- `$nor`, `$not`

### Element
- `$exists`
- `$type`

### Evaluation
- `$regex` *(uses the `regex` crate internally)*

### Array
- `$in`, `$nin`
- `$all`
- `$size`
- `$elemMatch`

> For the full reference, see [docs/query/01-query-operators.md](query/01-query-operators.md).

---

## 4. Cursor & Helpers

### `Cursor`
```rust
pub fn into_iter(self) -> impl Iterator<Item = Result<Document>>
```

### `Sorter::sort(docs, spec)`
Sorts in place. Ascending by default; `1` = asc, `-1` = desc.

### `Projection::apply(&doc, spec)`
Applies field inclusion/exclusion (not both).

---

## 5. FFI (C ABI)

```c
void* mongolite_create(const char* path);
void* mongolite_open (const char* path);
int    mongolite_close(void* db);

int    mongolite_insert (
    void* db,
    const char* collection,
    const char* json_doc,
    char* out_id,
    int   out_id_len
);

int    mongolite_find(
    void* db,
    const char* collection,
    const char* filter_json,
    char* out_json,
    int   out_json_len
);

int    mongolite_count(void* db, const char* collection);
int    mongolite_delete(void* db, const char* collection, const char* filter_json);
```

- All functions return `-1` on error.
- Strings are NUL-terminated `CStr`.
- On success, `mongolite_insert` writes the inserted `_id` hex-string
  to `out_id` (25 bytes including NUL).

---

## 6. Error Types

Defined in [`error.rs`](../crates/mongolite/src/error.rs), uses `thiserror`.

| Variant        | Description |
|----------------|-------------|
| `Io`           | Underlying file/IO errors. |
| `Bson`         | Failed BSON round-trip. |
| `Corrupted`    | File header or page checksum mismatch. |
| `CollectionNotFound` | Tried to access a nonexistent collection. |
| `DuplicateKey` | Primary key collision (reserved for future uniqueness enforcement). |
| `InvalidQuery` | Malformed or unsupported query/filter. |
| `InvalidUpdate` | Unsupported update operator or syntax. |
| `Locked`       | Placeholder for future transactional locking. |
| `Wal`          | Write-ahead log replay/append errors. |
