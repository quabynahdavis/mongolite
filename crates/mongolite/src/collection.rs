use bson::Document;

use crate::db::{deserialize_id, serialize_id, Database};
use crate::document::oid::ObjectId;
use crate::document::Document as MongoDocument;
use crate::error::Result;
use crate::query::QueryMatcher;
use crate::storage::btree::{BTree, BTreeConfig};

pub struct Collection<'a> {
    pub(crate) name: String,
    pub(crate) db: &'a Database,
}

pub struct InsertOneResult {
    pub inserted_id: ObjectId,
}

pub struct InsertManyResult {
    pub inserted_ids: Vec<ObjectId>,
}

pub struct UpdateResult {
    pub matched_count: u64,
    pub modified_count: u64,
}

pub struct DeleteResult {
    pub deleted_count: u64,
}

impl<'a> Collection<'a> {
    pub(crate) fn new(name: &str, db: &'a Database) -> Self {
        Self {
            name: name.to_string(),
            db,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn insert_one(&mut self, mut doc: Document) -> Result<InsertOneResult> {
        let id = match doc.get("_id") {
            Some(bson::Bson::ObjectId(id)) => ObjectId::from_bytes(id.bytes()),
            _ => {
                let id = ObjectId::new();
                doc.insert("_id", bson::Bson::ObjectId(bson::oid::ObjectId::from_bytes(*id.as_bytes())));
                id
            }
        };

        let mongo_doc = MongoDocument::from_bson(doc)?;
        let bytes = mongo_doc.to_bytes()?;
        let id_bytes = serialize_id(id);

        let index = self.get_or_create_index()?;
        index.insert(&id_bytes, &bytes)?;

        Ok(InsertOneResult { inserted_id: id })
    }

    pub fn insert_many(&mut self, docs: Vec<Document>) -> Result<InsertManyResult> {
        let mut ids = Vec::with_capacity(docs.len());
        for doc in docs {
            let result = self.insert_one(doc)?;
            ids.push(result.inserted_id);
        }
        Ok(InsertManyResult { inserted_ids: ids })
    }

    pub fn find(&self, filter: Option<Document>) -> Result<Vec<Document>> {
        let index = self.get_index()?;
        let entries = Box::leak(Box::new(index)).iter()?;
        let mut docs = Vec::new();
        let filter = filter.unwrap_or_default();
        for (_, value) in entries {
            let doc = MongoDocument::from_bytes(&value)?;
            let bson_doc = doc.into_bson();
            if QueryMatcher::matches(&bson_doc, &filter)? {
                docs.push(bson_doc.clone());
            }
        }
        Ok(docs)
    }

    pub fn find_with_options(
        &self,
        filter: Option<Document>,
        sort: Option<Document>,
        skip: Option<u64>,
        limit: Option<u64>,
        projection: Option<Document>,
    ) -> Result<Vec<Document>> {
        let mut docs = self.find(filter)?;

        if let Some(sort_spec) = sort {
            crate::cursor::Sorter::sort(&mut docs, &sort_spec);
        }

        let skip = skip.unwrap_or(0) as usize;
        if skip > 0 {
            docs = docs.into_iter().skip(skip).collect();
        }

        if let Some(lim) = limit {
            let lim = lim as usize;
            if lim < docs.len() {
                docs.truncate(lim);
            }
        }

        if let Some(proj) = projection {
            docs = docs
                .iter()
                .map(|d| crate::cursor::Projection::apply(d, &proj).unwrap())
                .collect();
        }

        Ok(docs)
    }

    pub fn find_one(&self, filter: Option<Document>) -> Result<Option<Document>> {
        let docs = self.find(filter)?;
        Ok(docs.into_iter().next())
    }

    pub fn update_one(&mut self, filter: Document, update: Document) -> Result<UpdateResult> {
        let id_bytes = self.filter_to_id(&filter)?;
        let index = self.get_or_create_index()?;

        if let Some(existing) = index.get(&id_bytes)? {
            let mut doc = MongoDocument::from_bytes(&existing)?;
            apply_update(&mut doc, &update)?;
            let new_bytes = doc.to_bytes()?;
            index.insert(&id_bytes, &new_bytes)?;
            Ok(UpdateResult {
                matched_count: 1,
                modified_count: 1,
            })
        } else {
            Ok(UpdateResult {
                matched_count: 0,
                modified_count: 0,
            })
        }
    }

    pub fn update_many(&mut self, filter: Document, update: Document) -> Result<UpdateResult> {
        self.update_one(filter, update)
    }

    pub fn delete_one(&mut self, filter: Document) -> Result<DeleteResult> {
        let id_bytes = self.filter_to_id(&filter)?;
        let index = self.get_or_create_index()?;
        if index.get(&id_bytes)?.is_some() {
            index.delete(&id_bytes)?;
            Ok(DeleteResult { deleted_count: 1 })
        } else {
            Ok(DeleteResult { deleted_count: 0 })
        }
    }

    pub fn delete_many(&mut self, filter: Document) -> Result<DeleteResult> {
        self.delete_one(filter)
    }

    pub fn count(&self, filter: Option<Document>) -> Result<u64> {
        let docs = self.find(filter)?;
        Ok(docs.len() as u64)
    }

    pub fn drop(&mut self) -> Result<bool> {
        self.db.drop_collection(&self.name)
    }

    fn get_or_create_index(&self) -> Result<&'static mut BTree<'static>> {
        let name_key = self.name.as_bytes().to_vec();
        let allocator = self.db.allocator;

        unsafe {
            if let Some(root_page_bytes) = (*self.db.catalog).get(&name_key)? {
                let root_page = u32::from_le_bytes(root_page_bytes.try_into().map_err(|_| {
                    crate::error::Error::Corrupted("invalid root page".into())
                })?);
                let index = BTree::open(&mut *allocator, root_page, BTreeConfig { order: 4 });
                return Ok(Box::leak(Box::new(index?)));
            }

            let index = BTree::new(&mut *allocator, BTreeConfig { order: 4 })?;
            let root_page = index.root_page();
            let root_bytes = root_page.to_le_bytes().to_vec();
            (*self.db.catalog).insert(&name_key, &root_bytes)?;

            Ok(Box::leak(Box::new(index)))
        }
    }

    fn get_index(&self) -> Result<&'static BTree<'static>> {
        let name_key = self.name.as_bytes().to_vec();
        let allocator = self.db.allocator;

        unsafe {
            let root_page_bytes = (*self.db.catalog).get(&name_key)?
                .ok_or_else(|| crate::error::Error::CollectionNotFound(self.name.clone()))?;
            let root_page = u32::from_le_bytes(root_page_bytes.try_into().map_err(|_| {
                crate::error::Error::Corrupted("invalid root page".into())
            })?);

            let index = BTree::open(&mut *allocator, root_page, BTreeConfig { order: 4 })?;
            Ok(Box::leak(Box::new(index)))
        }
    }

    fn filter_to_id(&self, filter: &Document) -> Result<Vec<u8>> {
        match filter.get("_id") {
            Some(bson::Bson::ObjectId(id)) => Ok(serialize_id(ObjectId::from_bytes(id.bytes()))),
            _ => Err(crate::error::Error::InvalidQuery("filter must contain _id".into())),
        }
    }
}

fn apply_update(doc: &mut MongoDocument, update: &Document) -> Result<()> {
    for (key, value) in update {
        if key.starts_with('$') {
            match key.as_str() {
                "$set" => {
                    if let Some(map) = value.as_document() {
                        for (k, v) in map {
                            doc.insert(k.clone(), v.clone());
                        }
                    }
                }
                "$unset" => {
                    if let Some(map) = value.as_document() {
                        for (k, _) in map {
                            doc.remove(k);
                        }
                    }
                }
                _ => return Err(crate::error::Error::InvalidUpdate(format!("unsupported operator: {}", key))),
            }
        } else {
            doc.insert(key.clone(), value.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use bson::doc;
    use tempfile::TempDir;

    use super::*;

    fn create_test_db() -> (Database, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");
        (Database::create(&path).unwrap(), dir)
    }

    #[test]
    fn test_insert_and_find() {
        let (mut db, _dir) = create_test_db();
        let mut coll = db.collection("users");

        coll.insert_one(doc! { "name": "Alice", "age": 30 }).unwrap();
        coll.insert_one(doc! { "name": "Bob", "age": 25 }).unwrap();

        let docs = coll.find(None).unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_insert_one_with_id() {
        let (mut db, _dir) = create_test_db();
        let mut coll = db.collection("users");

        let result = coll.insert_one(doc! { "name": "Alice" }).unwrap();
        assert!(result.inserted_id.timestamp() > 0);
    }

    #[test]
    fn test_find_one() {
        let (mut db, _dir) = create_test_db();
        let mut coll = db.collection("users");

        coll.insert_one(doc! { "name": "Alice" }).unwrap();
        let doc = coll.find_one(None).unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().get_str("name").unwrap(), "Alice");
    }

    #[test]
    fn test_update_one() {
        let (mut db, _dir) = create_test_db();
        let mut coll = db.collection("users");

        let result = coll.insert_one(doc! { "name": "Alice", "age": 30 }).unwrap();
        let id = result.inserted_id;

        let update_result = coll.update_one(
            doc! { "_id": bson::oid::ObjectId::from_bytes(*id.as_bytes()) },
            doc! { "$set": { "age": 31 } },
        ).unwrap();

        assert_eq!(update_result.matched_count, 1);
        assert_eq!(update_result.modified_count, 1);
    }

    #[test]
    fn test_delete_one() {
        let (mut db, _dir) = create_test_db();
        let mut coll = db.collection("users");

        let result = coll.insert_one(doc! { "name": "Alice" }).unwrap();
        let id = result.inserted_id;

        let delete_result = coll.delete_one(
            doc! { "_id": bson::oid::ObjectId::from_bytes(*id.as_bytes()) },
        ).unwrap();

        assert_eq!(delete_result.deleted_count, 1);
        assert_eq!(coll.count(None).unwrap(), 0);
    }

    #[test]
    fn test_count() {
        let (mut db, _dir) = create_test_db();
        let mut coll = db.collection("users");

        coll.insert_one(doc! { "name": "Alice" }).unwrap();
        coll.insert_one(doc! { "name": "Bob" }).unwrap();
        coll.insert_one(doc! { "name": "Charlie" }).unwrap();

        assert_eq!(coll.count(None).unwrap(), 3);
    }

    #[test]
    fn test_insert_many() {
        let (mut db, _dir) = create_test_db();
        let mut coll = db.collection("users");

        let docs = vec![
            doc! { "name": "Alice" },
            doc! { "name": "Bob" },
            doc! { "name": "Charlie" },
        ];

        let result = coll.insert_many(docs).unwrap();
        assert_eq!(result.inserted_ids.len(), 3);
        assert_eq!(coll.count(None).unwrap(), 3);
    }
}
