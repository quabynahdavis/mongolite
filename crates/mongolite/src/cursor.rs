use bson::{Bson, Document};

use crate::error::{Error, Result};

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

pub struct Projection;

impl Projection {
    pub fn apply(doc: &Document, projection: &Document) -> Result<Document> {
        if projection.is_empty() {
            return Ok(doc.clone());
        }

        let mut has_include = false;
        let mut has_exclude = false;

        for (key, value) in projection {
            if key == "_id" {
                continue;
            }
            if is_true(value) {
                has_include = true;
            } else {
                has_exclude = true;
            }
        }

        if has_include && has_exclude {
            return Err(Error::InvalidQuery(
                "projection cannot have a mix of inclusion and exclusion".into(),
            ));
        }

        let mut result = Document::new();

        if has_include {
            for (key, value) in projection {
                if key == "_id" {
                    continue;
                }
                if is_true(value) {
                    if let Some(val) = doc.get(key) {
                        result.insert(key.clone(), val.clone());
                    }
                }
            }
            let include_id = projection
                .get("_id")
                .map(|v| is_true(v))
                .unwrap_or(true);
            if include_id {
                if let Some(id) = doc.get("_id") {
                    result.insert("_id", id.clone());
                }
            }
        } else {
            for (key, val) in doc {
                let should_exclude = projection
                    .get(key)
                    .map(|v| !is_true(v))
                    .unwrap_or(false);
                if !should_exclude {
                    result.insert(key.clone(), val.clone());
                }
            }
        }

        Ok(result)
    }
}

fn is_true(value: &Bson) -> bool {
    match value {
        Bson::Int32(v) => *v != 0,
        Bson::Int64(v) => *v != 0,
        Bson::Double(v) => *v != 0.0,
        Bson::Boolean(v) => *v,
        Bson::Null => false,
        _ => true,
    }
}

pub struct Sorter;

impl Sorter {
    pub fn sort(docs: &mut [Document], sort_spec: &Document) {
        if sort_spec.is_empty() {
            return;
        }
        docs.sort_by(|a, b| {
            for (key, value) in sort_spec {
                let direction = match value {
                    Bson::Int32(v) => *v,
                    Bson::Int64(v) => *v as i32,
                    Bson::Double(v) => *v as i32,
                    _ => 1,
                };
                let a_val = a.get(key);
                let b_val = b.get(key);
                let cmp = cmp_bson_values(a_val, b_val);
                if cmp != std::cmp::Ordering::Equal {
                    return if direction >= 0 {
                        cmp
                    } else {
                        cmp.reverse()
                    };
                }
            }
            std::cmp::Ordering::Equal
        });
    }
}

fn cmp_bson_values(a: Option<&Bson>, b: Option<&Bson>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => match (a, b) {
            (Bson::Int32(a), Bson::Int32(b)) => a.cmp(b),
            (Bson::Int64(a), Bson::Int64(b)) => a.cmp(b),
            (Bson::Double(a), Bson::Double(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            (Bson::String(a), Bson::String(b)) => a.cmp(b),
            (Bson::Boolean(a), Bson::Boolean(b)) => a.cmp(b),
            (Bson::Null, Bson::Null) => std::cmp::Ordering::Equal,
            (Bson::Null, _) => std::cmp::Ordering::Less,
            (_, Bson::Null) => std::cmp::Ordering::Greater,
            (Bson::Int32(a), Bson::Int64(b)) => (*a as i64).cmp(b),
            (Bson::Int64(a), Bson::Int32(b)) => a.cmp(&(*b as i64)),
            (Bson::Int32(a), Bson::Double(b)) => (*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            (Bson::Double(a), Bson::Int32(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal),
            (Bson::Int64(a), Bson::Double(b)) => (*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            (Bson::Double(a), Bson::Int64(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal),
            _ => std::cmp::Ordering::Equal,
        },
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_include() {
        let doc = bson::doc! { "_id": "123", "name": "Alice", "age": 30, "email": "alice@example.com" };
        let projection = bson::doc! { "name": 1, "age": 1 };
        let result = Projection::apply(&doc, &projection).unwrap();
        assert_eq!(result.get_str("name").unwrap(), "Alice");
        assert_eq!(result.get_i32("age").unwrap(), 30);
        assert!(result.get("email").is_none());
        assert!(result.get("_id").is_some());
    }

    #[test]
    fn test_projection_exclude() {
        let doc = bson::doc! { "_id": "123", "name": "Alice", "age": 30, "email": "alice@example.com" };
        let projection = bson::doc! { "email": 0 };
        let result = Projection::apply(&doc, &projection).unwrap();
        assert_eq!(result.get_str("name").unwrap(), "Alice");
        assert_eq!(result.get_i32("age").unwrap(), 30);
        assert!(result.get("email").is_none());
    }

    #[test]
    fn test_projection_exclude_id() {
        let doc = bson::doc! { "_id": "123", "name": "Alice" };
        let projection = bson::doc! { "_id": 0, "name": 1 };
        let result = Projection::apply(&doc, &projection).unwrap();
        assert_eq!(result.get_str("name").unwrap(), "Alice");
        assert!(result.get("_id").is_none());
    }

    #[test]
    fn test_projection_mixed_error() {
        let doc = bson::doc! { "name": "Alice", "age": 30 };
        let projection = bson::doc! { "name": 1, "age": 0 };
        let result = Projection::apply(&doc, &projection);
        assert!(result.is_err());
    }

    #[test]
    fn test_sort_ascending() {
        let mut docs = vec![
            bson::doc! { "name": "Charlie" },
            bson::doc! { "name": "Alice" },
            bson::doc! { "name": "Bob" },
        ];
        let sort_spec = bson::doc! { "name": 1 };
        Sorter::sort(&mut docs, &sort_spec);
        assert_eq!(docs[0].get_str("name").unwrap(), "Alice");
        assert_eq!(docs[1].get_str("name").unwrap(), "Bob");
        assert_eq!(docs[2].get_str("name").unwrap(), "Charlie");
    }

    #[test]
    fn test_sort_descending() {
        let mut docs = vec![
            bson::doc! { "age": 25 },
            bson::doc! { "age": 35 },
            bson::doc! { "age": 30 },
        ];
        let sort_spec = bson::doc! { "age": -1 };
        Sorter::sort(&mut docs, &sort_spec);
        assert_eq!(docs[0].get_i32("age").unwrap(), 35);
        assert_eq!(docs[1].get_i32("age").unwrap(), 30);
        assert_eq!(docs[2].get_i32("age").unwrap(), 25);
    }

    #[test]
    fn test_sort_multi_field() {
        let mut docs = vec![
            bson::doc! { "age": 30, "name": "Bob" },
            bson::doc! { "age": 30, "name": "Alice" },
            bson::doc! { "age": 25, "name": "Charlie" },
        ];
        let sort_spec = bson::doc! { "age": 1, "name": 1 };
        Sorter::sort(&mut docs, &sort_spec);
        assert_eq!(docs[0].get_str("name").unwrap(), "Charlie");
        assert_eq!(docs[1].get_str("name").unwrap(), "Alice");
        assert_eq!(docs[2].get_str("name").unwrap(), "Bob");
    }

    #[test]
    fn test_sort_empty_spec() {
        let mut docs = vec![
            bson::doc! { "name": "Charlie" },
            bson::doc! { "name": "Alice" },
        ];
        let sort_spec = bson::doc! {};
        Sorter::sort(&mut docs, &sort_spec);
        assert_eq!(docs[0].get_str("name").unwrap(), "Charlie");
        assert_eq!(docs[1].get_str("name").unwrap(), "Alice");
    }
}
