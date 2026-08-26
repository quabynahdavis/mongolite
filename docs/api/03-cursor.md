# Cursor API

The `Cursor` struct enables lazy evaluation of query results, yielding documents
one-by-one like a streaming iterator.

## Importing

```rust
use mongolite::Cursor;
```

## Creating a Cursor

A `Cursor` is constructed from any existing `Vec<Document>` returned by
`Collection::find()`, typically via intermediate helpers in the `cursor` module
(e.g., `Cursor::new(docs)`).

## Iterator Trait

Cursors implement Rust’s standard `Iterator` interface:

```rust
let mut cursor = collection.find(None)?;
while let Some(doc) = cursor.next() {
    println!("{}", doc);
}
```

This allows easy integration with functional idioms:

```rust
let names = collection.find(None)?
    .filter(|d| d.get_str("active").unwrap_or(false))
    .map(|d| d.get_str("name").unwrap_or("?"))
    .collect::<Vec<_>>();
```

## Cursor States

| State       | Description                        |
|-------------|------------------------------------|
| Open        | Has unprocessed documents         |
| Closed      | End reached                       |
| Exhausted   | `.next()` returns `None`          |

There are no intermediate states; cursors do not buffer ahead beyond what
`find()` already loaded.

## Projection Helper

Located in `crates/mongolite/src/cursor.rs`:

```rust
pub fn project(doc: &Document, spec: &Document) -> Document
```

Selectively includes/excludes fields from a document.

Rules:
- Cannot mix inclusion and exclusion (except `_id`)
- Empty spec returns original unchanged
- Default behavior includes `_id` unless explicitly excluded

## Sorting Helper

```rust
pub fn sort(docs: &mut [Document], spec: &Document)
```

In-place sort of document slice according to specified field directions.

Supported types:
- Strings → lexicographic
- Integers (`i32`, `i64`) → numeric
- Doubles → numeric (with type coercion)
- Booleans → false before true

## Error Handling

Cursor construction does not perform filtering or sorting; errors arise only
during `Iterator::next()` if underlying data access fails. These surface as
`Error::Bson` or `Error::Corrupted`.

## Related

- [Collection API](02-collection.md) — where cursors originate
- [Query Operators](../query/01-query-operators.md) — how filters work
