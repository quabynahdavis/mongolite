//! Secondary index implementation for MongoLite.
//!
//! Indexes are stored as separate B+Trees within the database file, keyed by
//! the indexed field value. Each index entry maps `<field_value_bytes>` to
//! the serialized ObjectId of the document.

use crate::error::{Error, Result};
use crate::storage::btree::{BTree, BTreeConfig};
use crate::storage::allocator::Allocator;

/// Metadata describing an index on a collection.
#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub collection: String,
    pub field: String,
    pub unique: bool,
    /// Page ID of the BTree root for this index.
    pub root_page: u32,
}

/// An open handle to a secondary index.
pub struct Index {
    tree: *mut BTree<'static>,
    info: IndexInfo,
}

impl Index {
    /// Create a new index on the given collection/field.
    pub fn create(
        allocator: *mut Allocator<'static>,
        collection: &str,
        field: &str,
        unique: bool,
    ) -> Result<Index> {
        let tree = Box::leak(Box::new(BTree::new(unsafe { &mut *allocator }, BTreeConfig::default())?));
        Ok(Index {
            tree,
            info: IndexInfo {
                name: format!("idx_{}_{}", collection, field),
                collection: collection.to_string(),
                field: field.to_string(),
                unique,
                root_page: unsafe { (*tree).root_page() },
            },
        })
    }

    /// Open an existing index by its root page.
    pub fn open(
        allocator: *mut Allocator<'static>,
        info: IndexInfo,
    ) -> Result<Index> {
        let tree = Box::leak(Box::new(BTree::open(unsafe { &mut *allocator }, info.root_page, BTreeConfig::default())?));
        Ok(Index {
            tree,
            info,
        })
    }

    /// The name of this index.
    pub fn name(&self) -> &str {
        &self.info.name
    }

    /// The field this index covers.
    pub fn field(&self) -> &str {
        &self.info.field
    }

    /// Whether the index enforces uniqueness.
    pub fn is_unique(&self) -> bool {
        self.info.unique
    }

    /// Insert a (value, doc_id) pair into the index.
    pub fn insert(&mut self, value: &[u8], doc_id: &[u8]) -> Result<()> {
        if self.info.unique {
            if unsafe { (*self.tree).get(value)? }.is_some() {
                return Err(Error::DuplicateKey(format!(
                    "duplicate key violation on index '{}'", self.info.name
                )));
            }
        }
        unsafe { (*self.tree).insert(value, doc_id)?; }
        Ok(())
    }

    /// Remove a value->doc_id mapping from the index.
    pub fn remove(&mut self, value: &[u8]) -> Result<()> {
        unsafe { (*self.tree).delete(value)?; }
        Ok(())
    }

    /// Look up the doc_id for a given indexed value.
    pub fn get(&self, value: &[u8]) -> Result<Option<Vec<u8>>> {
        unsafe { (*self.tree).get(value) }
    }

    /// Iterate all (value, doc_id) pairs.
    pub fn iter(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        unsafe { (*self.tree).iter() }
    }

    /// Number of entries in the index.
    pub fn len(&self) -> Result<usize> {
        unsafe { (*self.tree).len() }
    }

    /// Internal access to the root page number.
    pub(crate) fn root_page(&self) -> u32 {
        self.info.root_page
    }
}

/// Encode a field path and collection name into a catalog key.
pub(crate) fn index_catalog_key(collection: &str, field: &str) -> Vec<u8> {
    format!("__index__:{collection}:{field}").into_bytes()
}

/// Serialize IndexInfo into bytes for catalog storage.
pub fn serialize_index_info(info: &IndexInfo) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(info.name.len() as u32).to_le_bytes());
    buf.extend_from_slice(info.name.as_bytes());
    buf.extend_from_slice(&(info.field.len() as u32).to_le_bytes());
    buf.extend_from_slice(info.field.as_bytes());
    buf.push(if info.unique { 1 } else { 0 });
    buf.extend_from_slice(&info.root_page.to_le_bytes());
    Ok(buf)
}

/// Deserialize IndexInfo from bytes stored in the catalog.
pub fn deserialize_index_info(bytes: &[u8]) -> Result<IndexInfo> {
    if bytes.len() < 16 {
        return Err(Error::Corrupted("index info too short".into()));
    }
    let mut offset = 0;

    let name_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;

    let name = String::from_utf8(bytes[offset..offset + name_len].to_vec())
        .map_err(|_| Error::Corrupted("invalid index name".into()))?;
    offset += name_len;

    if offset + 4 > bytes.len() {
        return Err(Error::Corrupted("truncated index field length".into()));
    }
    let field_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;

    let field = String::from_utf8(bytes[offset..offset + field_len].to_vec())
        .map_err(|_| Error::Corrupted("invalid index field".into()))?;
    offset += field_len;

    if offset + 1 > bytes.len() {
        return Err(Error::Corrupted("truncated index uniqueness flag".into()));
    }
    let unique = bytes[offset] != 0;
    offset += 1;

    if offset + 4 > bytes.len() {
        return Err(Error::Corrupted("truncated index root page".into()));
    }
    let root_page = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());

    Ok(IndexInfo {
        name,
        collection: String::new(),
        field,
        unique,
        root_page,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::file::{File, DEFAULT_PAGE_SIZE};
    use crate::storage::allocator::Allocator;
    use tempfile::TempDir;

    fn create_test_allocator() -> (*mut Allocator<'static>, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");
        let file = Box::leak(Box::new(File::create(&path, DEFAULT_PAGE_SIZE).unwrap()));
        (Allocator::new(unsafe { &mut *file }) as *mut _, dir)
    }

    #[test]
    fn test_index_create_and_insert() {
        let (alloc_ptr, _dir) = create_test_allocator();
        let mut idx = Index::create(alloc_ptr, "users", "name", true).unwrap();

        idx.insert(b"Alice", b"doc_id_1").unwrap();
        let result = idx.get(b"Alice").unwrap();
        assert_eq!(result, Some(b"doc_id_1".to_vec()));
    }

    #[test]
    fn test_unique_violation() {
        let (alloc_ptr, _dir) = create_test_allocator();
        let mut idx = Index::create(alloc_ptr, "users", "email", true).unwrap();

        idx.insert(b"alice@example.com", b"doc1").unwrap();
        let result = idx.insert(b"alice@example.com", b"doc2");
        assert!(matches!(result, Err(Error::DuplicateKey(_))));
    }

    #[test]
    fn test_non_unique_allows_duplicates() {
        let (alloc_ptr, _dir) = create_test_allocator();
        let mut idx = Index::create(alloc_ptr, "logs", "level", false).unwrap();

        idx.insert(b"info", b"doc1").unwrap();
        idx.insert(b"info", b"doc2").unwrap();
        assert_eq!(idx.len().unwrap(), 2);
    }

    #[test]
    fn test_index_remove() {
        let (alloc_ptr, _dir) = create_test_allocator();
        let mut idx = Index::create(alloc_ptr, "tags", "label", true).unwrap();

        idx.insert(b"rust", b"doc1").unwrap();
        idx.remove(b"rust").unwrap();
        assert!(idx.get(b"rust").unwrap().is_none());
    }

    #[test]
    fn test_index_info_serialization() {
        let info = IndexInfo {
            name: "idx_users_name".to_string(),
            collection: "users".to_string(),
            field: "name".to_string(),
            unique: true,
            root_page: 42,
        };
        let bytes = serialize_index_info(&info).unwrap();
        let deserialized = deserialize_index_info(&bytes).unwrap();
        assert_eq!(deserialized.name, info.name);
        assert_eq!(deserialized.field, info.field);
        assert_eq!(deserialized.unique, info.unique);
        assert_eq!(deserialized.root_page, info.root_page);
    }
}