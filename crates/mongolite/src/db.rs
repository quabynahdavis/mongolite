use std::path::Path;

use crate::document::oid::ObjectId;
use crate::error::{Error, Result};
use crate::storage::allocator::Allocator;
use crate::storage::btree::{BTree, BTreeConfig};
use crate::storage::file::{File, DEFAULT_PAGE_SIZE};

pub use crate::collection::{
    Collection, DeleteResult, InsertManyResult, InsertOneResult, UpdateResult,
};
pub use crate::cursor::Cursor;

pub struct Database {
    file: &'static mut File,
    pub(crate) allocator: *mut Allocator<'static>,
    pub(crate) catalog: *mut BTree<'static>,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        Self::from_file(file)
    }

    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::create(path.as_ref(), DEFAULT_PAGE_SIZE)?;
        Self::from_file(file)
    }

    pub fn open_or_create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        match std::fs::metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::create(path),
            Err(e) => Err(Error::Io(e)),
            Ok(meta) if meta.len() == 0 => {
                let file = File::init_empty(path, DEFAULT_PAGE_SIZE)?;
                Self::from_file(file)
            }
            Ok(_) => Self::open(path),
        }
    }

    fn from_file(file: File) -> Result<Self> {
        let file = Box::leak(Box::new(file));
        let file_ptr: *mut File = file;
        let allocator = Box::leak(Box::new(Allocator::new(unsafe { &mut *file_ptr })));
        let alloc_ptr: *mut Allocator = allocator;

        let catalog = unsafe {
            if (*file_ptr).header().catalog_root_page != 0 {
                BTree::open(
                    &mut *alloc_ptr,
                    (*file_ptr).header().catalog_root_page,
                    BTreeConfig::default(),
                )
            } else {
                BTree::new(&mut *alloc_ptr, BTreeConfig::default())
            }
        }?;

        let catalog = Box::leak(Box::new(catalog));
        let catalog_ptr: *mut BTree = catalog;

        unsafe {
            let header = (*file_ptr).header_mut();
            header.catalog_root_page = (*catalog_ptr).root_page();
            header.checksum = header.compute_checksum();
        }

        Ok(Self {
            file,
            allocator: alloc_ptr,
            catalog: catalog_ptr,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn allocator_mut(&mut self) -> &mut Allocator<'static> {
        unsafe { &mut *self.allocator }
    }

    #[allow(dead_code)]
    pub(crate) fn catalog_mut(&mut self) -> &mut BTree<'static> {
        unsafe { &mut *self.catalog }
    }

    pub fn collection(&mut self, name: &str) -> Collection<'_> {
        Collection::new(name, self)
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

    #[allow(dead_code)]
    pub(crate) fn file_mut(&mut self) -> &mut File {
        self.file
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = self.file.flush();
    }
}

pub(crate) fn serialize_id(id: ObjectId) -> Vec<u8> {
    id.as_bytes().to_vec()
}

#[allow(dead_code)]
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
    use std::fs::File as StdFile;
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

    #[test]
    fn test_open_or_create_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");
        StdFile::create(&path).unwrap();

        let mut db = Database::open_or_create(&path).unwrap();
        let mut coll = db.collection("users");
        coll.insert_one(bson::doc! { "name": "Alice" }).unwrap();
        let docs = coll.find(None).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].get_str("name").unwrap(), "Alice");
    }

    #[test]
    fn test_open_or_create_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        {
            let mut db = Database::create(&path).unwrap();
            db.collection("users")
                .insert_one(bson::doc! { "name": "Bob" })
                .unwrap();
            db.flush().unwrap();
        }

        {
            let db = Database::open_or_create(&path).unwrap();
            assert!(db.list_collections().unwrap().contains(&"users".to_string()));
        }

        {
            let db = Database::open(&path).unwrap();
            assert!(db.list_collections().unwrap().contains(&"users".to_string()));
        }
    }

    #[test]
    fn test_open_or_create_missing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let db = Database::open_or_create(&path).unwrap();
        assert!(db.list_collections().unwrap().is_empty());
        assert!(path.exists());
    }

    #[test]
    fn test_open_or_create_corrupt() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");
        std::fs::write(&path, b"not a valid database header!!!!!!!!!!!!!!").unwrap();

        let result = Database::open_or_create(&path);
        assert!(matches!(result, Err(Error::Corrupted(_))));
    }
}
