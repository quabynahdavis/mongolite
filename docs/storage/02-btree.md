# B+tree Indexes

MongoLite uses B+tree indexes for efficient document retrieval. B+trees provide logarithmic-time lookups, insertions, and deletions, as well as efficient range scans.

## Structure

A B+tree is a balanced tree where:

- **Internal nodes** contain keys and pointers to child nodes.
- **Leaf nodes** contain keys and pointers to actual data (document locations).
- **All leaf nodes** are linked together in a doubly-linked list for range scans.

```
                ┌─────────┐
                │   50    │  ← Internal node
                └────┬────┘
           ┌─────────┴─────────┐
     ┌─────┴─────┐       ┌─────┴─────┐
     │  20 │ 35  │       │  70 │ 90  │  ← Internal nodes
     └──┬──┴──┬──┘       └──┬──┴──┬──┘
   ┌────┘     └──┐    ┌────┘     └──┐
┌──┴──┐      ┌──┴──┐┌──┴──┐      ┌──┴──┐
│10│15│      │25│30││55│60│      │75│80│  ← Leaf nodes
└──┬──┘      └──┬──┘└──┬──┘      └──┬──┘
   │            │      │            │
   └────────────┴──────┴────────────┘
         Doubly-linked list
```

## Properties

| Property | Value |
|----------|-------|
| Order (fanout) | Variable; depends on key size and page size |
| Height | O(log N) where N is the number of entries |
| Leaf linkage | Doubly-linked list for range scans |
| Balance | All leaves at the same depth |

## Node Layout

### Internal Node

```
┌─────────────────────────────────────────┐
│ Page Header                             │
├─────────────────────────────────────────┤
│ Number of keys (2 bytes)                │
│ Rightmost child pointer (4 bytes)       │
├─────────────────────────────────────────┤
│ Child 0 pointer (4 bytes)               │
│ Key 1 (variable length)                 │
│ Child 1 pointer (4 bytes)               │
│ Key 2 (variable length)                 │
│ ...                                     │
│ Child N pointer (4 bytes)               │
└─────────────────────────────────────────┘
```

### Leaf Node

```
┌─────────────────────────────────────────┐
│ Page Header                             │
├─────────────────────────────────────────┤
│ Number of entries (2 bytes)             │
│ Left sibling page ID (4 bytes)          │
│ Right sibling page ID (4 bytes)         │
├─────────────────────────────────────────┤
│ Key 1 │ Document pointer (8 bytes)      │
│ Key 2 │ Document pointer                │
│ ...                                    │
│ Key N │ Document pointer                │
└─────────────────────────────────────────┘
```

## Operations

### Lookup

1. Start at the root node.
2. At each internal node, find the smallest key greater than the search key.
3. Follow the corresponding child pointer.
4. Repeat until a leaf node is reached.
5. Search the leaf node for the exact key.

**Time complexity:** O(log N)

### Insertion

1. Navigate to the appropriate leaf node.
2. Insert the key-value pair in sorted order.
3. If the leaf is full, **split** it:
   - Create a new leaf node.
   - Move half the entries to the new node.
   - Insert a separator key in the parent.
4. If the parent is full, split recursively up to the root.
5. If the root splits, create a new root (tree grows in height).

**Time complexity:** O(log N)

### Deletion

1. Navigate to the leaf containing the key.
2. Remove the key-value pair.
3. If the leaf is less than half full:
   - Try to **borrow** an entry from a sibling.
   - If borrowing is not possible, **merge** with a sibling.
4. Update parent keys if necessary.

**Time complexity:** O(log N)

### Range Scan

1. Navigate to the first leaf node containing the range start.
2. Scan forward through the linked leaf nodes.
3. Stop when the range end is reached.

**Time complexity:** O(log N + K) where K is the number of results.

## Splitting

When a node becomes full (exceeds the page size), it is split:

```
Before split (leaf with 4 entries, max 3):
┌─────────────────┐
│ 10 │ 20 │ 30 │ 40 │
└─────────────────┘

After split:
┌──────────┐         ┌──────────┐
│ 10 │ 20  │ ──────▶ │ 30 │ 40  │
└──────────┘         └──────────┘
     │
     │ separator key: 30
     ▼
  Parent updated
```

## Merging

When a node becomes less than half full after deletion, it may merge with a sibling:

```
Before merge:
┌──────────┐         ┌──────────┐
│ 10 │ 20  │ ──────▶ │ 30 │     │  (underflow)
└──────────┘         └──────────┘

After merge:
┌──────────────────────┐
│ 10 │ 20 │ 30         │
└──────────────────────┘
```

## Index Types

### Single-Field Index

```rust
// Index on a single field
collection.create_index(doc! { "age": 1 }, IndexOptions::new())?;
```

### Compound Index

```rust
// Index on multiple fields
collection.create_index(
    doc! { "last_name": 1, "first_name": 1 },
    IndexOptions::new(),
)?;
```

### Unique Index

```rust
// Enforce uniqueness
collection.create_index(
    doc! { "email": 1 },
    IndexOptions::new().unique(true),
)?;
```

### Multikey Index (Array Fields)

When indexing a field that contains an array, MongoLite creates a **multikey index** — one entry per array element:

```rust
// Document: { tags: ["rust", "database"] }
// Index on "tags" creates two entries:
//   "rust" → document pointer
//   "database" → document pointer
```

## Index Selectivity

The query planner uses index statistics to choose the most efficient index:

- **Selective indexes** (many distinct values) are preferred for equality matches.
- **Compound indexes** follow the **ESR rule**: Equality, Sort, Range fields in that order.

## Concurrency

B+tree operations use **latch coupling** (crabbing):

1. Pin the parent node with a shared latch.
2. Pin the child node with the appropriate latch.
3. Release the parent latch if the child is safe (not full/empty).
4. Continue down the tree.

This allows concurrent reads and minimizes write contention.

## Future Enhancements

- **Bulk loading** — Efficient index construction for large data imports.
- **Partial indexes** — Index only documents matching a filter expression.
- **TTL indexes** — Automatic expiration of documents after a time period.
- **Geospatial indexes** — 2dsphere indexes for location queries.
