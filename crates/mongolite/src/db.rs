use std::path::Path;

use crate::document::oid::ObjectId;
use crate::error::{Error, Result};
use crate::storage::allocator::Allocator;
use crate::storage::btree::{BTree, BTreeConfig};
use crate::storage::file::{File, DEFAULT_PAGE_SIZE};
use crate::storage::index::{Index, IndexInfo, index_catalog_key, serialize_index_info, deserialize_index_info};

pub use crate::collection::{Collection, DeleteResult, InsertManyResult, InsertOneResult, UpdateResult};
pub use crate::cursor::Cursor;

pub struct Database {
    file: Box<File>,
    pub(crate) allocator: Box<Allocator<'static>>,
    pub(crate) catalog: Box<BTree<'static>>,
}

impl Database {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = Box::new(File::create(path.as_ref(), DEFAULT_PAGE_SIZE)?);
        let allocator = Box::new(Allocator::new(unsafe { &mut *file.as_mut() }));
        let catalog = Box::new(BTree::new(&mut *allocator, BTreeConfig::default())?);

        Ok(Self {
            file,
            allocator,
            catalog,
        })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = Box::new(File::open(path.as_ref())?);
        Self::from_file(file)
    }

    fn from_file(file: Box<File>) -> Result<Self> {
        let mut allocator = Box::new(Allocator::new(unsafe { &mut *file.as_mut() }));
        let catalog_root_page = unsafe { file.as_ref() }.header().catalog_root_page;
        let catalog = if catalog_root_page != 0 {
            Box::new(BTree::open(
                &mut *allocator,
                catalog_root_page,
                BTreeConfig::default(),
            )?)
        } else {
            Box::new(BTree::new(&mut *allocator, BTreeConfig::default())?)
        };

        Ok(Self {
            file,
            allocator,
            catalog,
        })
    }

    pub fn collection(&mut self, name: &str) -> Collection<'_> {
        Collection::new(name, self)
    }

    /// Create a new secondary index on a collection's field.
    ///
    /// The index metadata is persisted in the database catalog so it survives
    /// restarts. After creation, use `get_index` to access it.
    pub fn create_index(
        &mut self,
        collection: &str,
        field: &str,
        unique: bool,
    ) -> Result<()> {
        let idx = Index::create(&mut *self.allocator as *mut _, collection, field, unique)?;
        let root_page = idx.info.root_page;

        // Store index metadata in the catalog
        let catalog_key = index_catalog_key(collection, field);
        let info = IndexInfo {
            name: format!("idx_{}_{}", collection, field),
            collection: collection.to_string(),
            field: field.to_string(),
            unique,
            root_page,
        };
        let info_bytes = serialize_index_info(&info)?;
        unsafe {
            (*self.catalog).insert(&catalog_key, &info_bytes)?;
        }

        Ok(())
    }

    /// Create a new secondary index on a collection field.
    ///
    /// The index metadata is persisted in the database catalog so it survives
    /// restarts.
    pub fn create_index(
        &mut self,
        collection: &str,
        field: &str,
        unique: bool,
    ) -> Result<()> {
        let idx = Index::create(&mut *self.allocator as *mut _, collection, field, unique)?;
        let root_page = idx.info.root_page;

        // Store index metadata in the catalog
        let catalog_key = index_catalog_key(collection, field);
        let info = IndexInfo {
            name: format!("idx_{}_{}", collection, field),
            collection: collection.to_string(),
            field: field.to_string(),
            unique,
            root_page,
        };
        let info_bytes = serialize_index_info(&info)?;
        unsafe {
            (*self.catalog).insert(&catalog_key, &info_bytes)?;
        }

        Ok(())
    }

    /// Retrieve an index handle for an existing collection field.
    pub fn get_index(
        &mut self,
        collection: &str,
        field: &str,
    ) -> Result<Option<Index<'static>>> {
        let catalog_key = index_catalog_key(collection, field);
        let info_bytes = unsafe { (*self.catalog).get(&catalog_key)? };

        match info_bytes {
            Some(bytes) => {
                let mut info = deserialize_index_info(&bytes)?;
                info.collection = collection.to_string();
                let idx = Index::open(&mut *self.allocator, info)?;
                Ok(Some(idx))
            }
            None => Ok(None),
        }
    }

    /// List all index names on a collection.
    pub fn list_indexes(&self, collection: &str) -> Result<Vec<String>> {
        let entries = unsafe { &*self.catalog }.iter()?;
        let mut names = Vec::new();
        let prefix = format!("__index__:{collection}:");
        for (key, _) in entries {
            if let Ok(key_str) = String::from_utf8(key) {
                if key_str.starts_with(&prefix) {
                    if let Some(rest) = key_str.strip_prefix("__index__:") {
                        if let Some(field_part) = rest.strip_prefix(&format!("{collection}:")) {
                            names.push(format!("idx_{}_{}", collection, field_part));
                        }
                    }
                }
            }
        }
        Ok(names)
    }

    pub fn list_collections(&self) -> Result<Vec<String>> {
        let entries = unsafe { &*self.catalog }.iter()?;
        let mut names = Vec::new();
        for (key, _) in entries {
            if let Ok(name) = String::from_utf8(key) {
                names.push(name);
            }
        }
        Ok(names)
    }

    pub fn drop_collection(&self, name: &str) -> Result<bool> {
        let key = name.as_bytes().to_vec();
        unsafe {
            if (*self.catalog).get(&key)?.is_some() {
                (*self.catalog).delete(&key)?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    pub fn flush(&mut self) -> Result<()> {
        self.file.flush()
    }

    pub(crate) fn allocator_mut(&mut self) -> &mut Allocator<'static> {
        &mut *self.allocator
    }

    pub(crate) fn catalog_mut(&mut self) -> &mut BTree<'static> {
        &mut *self.catalog
    }

    pub(crate) fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    pub fn close(&mut self) -> Result<()> {
        // Flush any pending writes before the Database is destroyed.
        self.file.flush()?;
        Ok(())
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        // Ensure durability: flush to disk before the file handle is released.
        let _ = self.close();
    }
}

pub(crate) fn serialize_id(id: ObjectId) -> Vec<u8> {
    id.as_bytes().to_vec()
}

pub(crate) fn deserialize_id(bytes: &[u8]) -> Result<ObjectId> {
    if bytes.len() != 12 {
        return Err(Error::Corrupted("invalid _id length".into()));
    }
    let mut arr = [0u8; 12];
    arr.copy_from_slice(bytes);
    Ok(ObjectId::from_bytes(arr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use tempfile::TempDir;

    fn create_test_db() -> (Database, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");
        let db = Database::create(&path).unwrap();
        (db, dir)
    }

    #[test]
    fn test_create_and_open() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        {
            let _db = Database::create(&path).unwrap();
        }

        {
            let db = Database::open(&path).unwrap();
            assert!(db.list_collections().unwrap().is_empty());
        }
    }

    #[test]
    fn test_create_collection() {
        let (mut db, _dir) = create_test_db();
        let coll = db.collection("users");
        assert_eq!(coll.name(), "users");
    }
}