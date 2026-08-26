# B+Tree Index

MongoLite uses a custom B+Tree implementation (`crates/mongolite/src/storage/btree.rs`)
to store both the internal catalog of collections and each collection’s documents keyed
by `_id`.

## Data Model

- Keys and values are arbitrary byte vectors (`Vec<u8>`).
- Each tree has a root page managed by the `BTree<'a>` struct.
- Trees are typed through `BTreeConfig` which sets the tree's fan-out/order.

## Configuration

```rust
pub struct BTreeConfig {
    pub order: usize, // Max number of keys in an internal/leaf node
}
```

The default `order` is computed dynamically via `max_order(page_size)`
to ensure each node fits within a single page.

## Leaf Node Layout

```
[PageHeader (8B)]
[ num_keys : u32 ]
[ next_leaf_page : u32 ]
[[ key_len:u32 ][ key ] [ value_len:u32 ][ value ]] * num_keys
```

All leaf pages form a doubly-linked chain through `next_leaf_page`.

## Internal Node Layout

```
[PageHeader (8B)]
[ num_keys : u32 ]
[ child_page_id:u32 ] [ [ key_len:u32 ][ key ] [ child_page_id:u32 ] ] * num_keys
```

An internal node contains exactly `num_keys + 1` child pointers.

## Insertion

1. Descend the tree to the appropriate leaf node by comparing keys.
2. If the leaf is full (`num_keys >= order`), split it in half and promote the
   median key upward.
3. If internal nodes overflow during promotion, recursively split them.
4. Splits may propagate up to the root; a new root is allocated if necessary.

## Deletion

1. Descend to the leaf node containing the target key.
2. Remove the `(key, value)` pair from the payload section.
3. Compact remaining entries downward.
4. No rebalancing is implemented yet — deletion shrinks nodes monotonically.
   This can cause fragmentation but avoids complex underflow logic in early stages.

## Searching / Iteration

- Point lookups use standard binary search on keys.
- Iteration walks the leaf chain left-to-right, providing sorted traversal.
- Range queries perform a bounded scan starting at the first key ≥ `start`.

## Statistics

Each B+Tree exposes `BTreeStats`:

```rust
pub struct BTreeStats {
    pub height: usize,         // Tree depth
    pub leaf_nodes: usize,     // Number of leaf pages
    pub internal_nodes: usize, // Number of internal pages
    pub total_keys: usize,     // Total documents across all leaves
}
```

## Example Usage

```rust
use crate::storage::btree::{BTree, BTreeConfig};

let mut tree = BTree::new(&mut allocator, BTreeConfig { order: 4 })?;

// Insert
tree.insert(b"hello", b"world")?;

// Lookup
let value = tree.get(b"hello")?;

// Iterate keys in order
let entries = tree.iter()?;

// Remove
tree.delete(b"hello")?;
```

---

Next: [WAL format](03-wal.md)
