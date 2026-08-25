pub use bson::oid::ObjectId;

/// The ID type used for documents
pub type DocumentId = ObjectId;

pub type Value = bson::Bson;
pub type Field = (String, bson::Bson);

const ID_KEY: &str = "_id";

/// A document with its associated metadata
#[derive(Debug, Clone)]
pub struct Document {
    inner: bson::Document,
}

impl Document {
    pub fn new() -> Self {
        Self {
            inner: bson::Document::new(),
        }
    }

    pub fn with_id(id: DocumentId) -> Self {
        let mut doc = Self::new();
        doc.set_id(id);
        doc
    }

    pub fn from_bson(inner: bson::Document) -> Result<Self, crate::error::Error> {
        Ok(Self { inner })
    }

    pub fn to_bson(&self) -> &bson::Document {
        &self.inner
    }

    pub fn into_bson(self) -> bson::Document {
        self.inner
    }

    pub fn id(&self) -> Option<DocumentId> {
        self.inner.get(ID_KEY).and_then(|v| v.as_object_id())
    }

    pub fn set_id(&mut self, id: DocumentId) {
        self.inner.insert(ID_KEY, id);
    }

    pub fn generate_id(&mut self) -> DocumentId {
        let id = ObjectId::new();
        self.set_id(id);
        id
    }

    pub fn get(&self, key: &str) -> Option<&bson::Bson> {
        self.inner.get(key)
    }

    pub fn insert(&mut self, key: String, value: bson::Bson) {
        self.inner.insert(key, value);
    }

    pub fn remove(&mut self, key: &str) -> Option<bson::Bson> {
        self.inner.remove(key)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, crate::error::Error> {
        bson::to_vec(&self.inner).map_err(|e| crate::error::Error::Bson(e.to_string()))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::error::Error> {
        let inner = bson::from_slice(bytes)
            .map_err(|e| crate::error::Error::Bson(e.to_string()))?;
        Ok(Self { inner })
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl From<bson::Document> for Document {
    fn from(inner: bson::Document) -> Self {
        Self { inner }
    }
}

impl From<Document> for bson::Document {
    fn from(doc: Document) -> Self {
        doc.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_document_is_empty() {
        let doc = Document::new();
        assert!(doc.is_empty());
        assert_eq!(doc.len(), 0);
    }

    #[test]
    fn test_with_id() {
        let id = ObjectId::new();
        let doc = Document::with_id(id);

        assert_eq!(doc.id(), Some(id));
        assert_eq!(doc.len(), 1);
    }

    #[test]
    fn test_from_bson() {
        let mut inner = bson::Document::new();
        inner.insert("name", "test");

        let doc = Document::from_bson(inner).unwrap();
        assert_eq!(doc.get("name").unwrap().as_str(), Some("test"));
    }

    #[test]
    fn test_to_bson() {
        let mut doc = Document::new();
        doc.insert("key".to_string(), bson::Bson::String("value".to_string()));

        let bson = doc.to_bson();
        assert_eq!(bson.get_str("key").unwrap(), "value");
    }

    #[test]
    fn test_into_bson() {
        let mut doc = Document::new();
        doc.insert("key".to_string(), bson::Bson::String("value".to_string()));

        let bson: bson::Document = doc.into();
        assert_eq!(bson.get_str("key").unwrap(), "value");
    }

    #[test]
    fn test_set_and_get_id() {
        let mut doc = Document::new();
        let id = ObjectId::new();

        assert!(doc.id().is_none());

        doc.set_id(id);
        assert_eq!(doc.id(), Some(id));
    }

    #[test]
    fn test_generate_id() {
        let mut doc = Document::new();
        let id = doc.generate_id();

        assert_eq!(doc.id(), Some(id));
        assert_eq!(doc.len(), 1);
    }

    #[test]
    fn test_get() {
        let mut doc = Document::new();
        doc.insert("name".to_string(), bson::Bson::String("Alice".to_string()));

        assert_eq!(doc.get("name").unwrap().as_str(), Some("Alice"));
        assert!(doc.get("missing").is_none());
    }

    #[test]
    fn test_insert() {
        let mut doc = Document::new();
        doc.insert("age".to_string(), bson::Bson::Int32(30));

        assert_eq!(doc.get("age").unwrap().as_i32(), Some(30));
        assert_eq!(doc.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut doc = Document::new();
        doc.insert("temp".to_string(), bson::Bson::String("value".to_string()));

        let removed = doc.remove("temp");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().as_str(), Some("value"));
        assert!(doc.get("temp").is_none());
        assert!(doc.is_empty());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut doc = Document::new();
        let removed = doc.remove("missing");
        assert!(removed.is_none());
    }

    #[test]
    fn test_to_bytes_from_bytes_roundtrip() {
        let mut doc = Document::new();
        doc.insert("name".to_string(), bson::Bson::String("test".to_string()));
        doc.insert("count".to_string(), bson::Bson::Int32(42));

        let bytes = doc.to_bytes().unwrap();
        let restored = Document::from_bytes(&bytes).unwrap();

        assert_eq!(restored.get("name").unwrap().as_str(), Some("test"));
        assert_eq!(restored.get("count").unwrap().as_i32(), Some(42));
    }

    #[test]
    fn test_from_bytes_invalid() {
        let result = Document::from_bytes(b"invalid bson data");
        assert!(result.is_err());
    }

    #[test]
    fn test_default() {
        let doc: Document = Default::default();
        assert!(doc.is_empty());
    }

    #[test]
    fn test_from_bson_document() {
        let mut inner = bson::Document::new();
        inner.insert("key", "value");

        let doc: Document = inner.into();
        assert_eq!(doc.get("key").unwrap().as_str(), Some("value"));
    }

    #[test]
    fn test_from_document_to_bson() {
        let mut doc = Document::new();
        doc.insert("key".to_string(), bson::Bson::String("value".to_string()));

        let bson: bson::Document = doc.into();
        assert_eq!(bson.get_str("key").unwrap(), "value");
    }

    #[test]
    fn test_nested_document() {
        let mut inner = bson::Document::new();
        inner.insert("street", "123 Main St");
        inner.insert("city", "Springfield");

        let mut doc = Document::new();
        doc.insert("address".to_string(), bson::Bson::Document(inner));

        let address = doc.get("address").unwrap().as_document().unwrap();
        assert_eq!(address.get_str("street").unwrap(), "123 Main St");
        assert_eq!(address.get_str("city").unwrap(), "Springfield");
    }

    #[test]
    fn test_multiple_fields() {
        let mut doc = Document::new();
        doc.insert("a".to_string(), bson::Bson::Int32(1));
        doc.insert("b".to_string(), bson::Bson::Int32(2));
        doc.insert("c".to_string(), bson::Bson::Int32(3));

        assert_eq!(doc.len(), 3);
        assert_eq!(doc.get("a").unwrap().as_i32(), Some(1));
        assert_eq!(doc.get("b").unwrap().as_i32(), Some(2));
        assert_eq!(doc.get("c").unwrap().as_i32(), Some(3));
    }
}
