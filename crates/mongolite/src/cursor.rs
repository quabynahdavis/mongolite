use bson::Document;

pub struct Cursor {
    documents: Vec<Document>,
    position: usize,
}

impl Cursor {
    pub fn new(documents: Vec<Document>) -> Self {
        Self {
            documents,
            position: 0,
        }
    }
}

impl Iterator for Cursor {
    type Item = Document;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.documents.len() {
            let doc = self.documents[self.position].clone();
            self.position += 1;
            Some(doc)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bson::doc;

    #[test]
    fn test_cursor_iter() {
        let docs = vec![
            doc! { "name": "Alice" },
            doc! { "name": "Bob" },
        ];

        let mut cursor = Cursor::new(docs);
        assert!(cursor.next().is_some());
        assert!(cursor.next().is_some());
        assert!(cursor.next().is_none());
    }

    #[test]
    fn test_cursor_empty() {
        let cursor = Cursor::new(vec![]);
        let count = cursor.count();
        assert_eq!(count, 0);
    }
}
