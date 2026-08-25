use bson::{Bson, Document};

use crate::error::{Error, Result};

pub struct QueryMatcher;

impl QueryMatcher {
    pub fn matches(doc: &Document, filter: &Document) -> Result<bool> {
        for (key, value) in filter {
            if key.starts_with('$') {
                match key.as_str() {
                    "$and" => {
                        let arr = value.as_array().ok_or_else(|| {
                            Error::InvalidQuery("$and requires array".into())
                        })?;
                        for sub_filter in arr {
                            let sub_doc = sub_filter.as_document().ok_or_else(|| {
                                Error::InvalidQuery("$and elements must be documents".into())
                            })?;
                            if !Self::matches(doc, sub_doc)? {
                                return Ok(false);
                            }
                        }
                    }
                    "$or" => {
                        let arr = value.as_array().ok_or_else(|| {
                            Error::InvalidQuery("$or requires array".into())
                        })?;
                        let mut any_match = false;
                        for sub_filter in arr {
                            let sub_doc = sub_filter.as_document().ok_or_else(|| {
                                Error::InvalidQuery("$or elements must be documents".into())
                            })?;
                            if Self::matches(doc, sub_doc)? {
                                any_match = true;
                                break;
                            }
                        }
                        if !any_match {
                            return Ok(false);
                        }
                    }
                    "$nor" => {
                        let arr = value.as_array().ok_or_else(|| {
                            Error::InvalidQuery("$nor requires array".into())
                        })?;
                        for sub_filter in arr {
                            let sub_doc = sub_filter.as_document().ok_or_else(|| {
                                Error::InvalidQuery("$nor elements must be documents".into())
                            })?;
                            if Self::matches(doc, sub_doc)? {
                                return Ok(false);
                            }
                        }
                    }
                    "$not" => {
                        let sub_doc = value.as_document().ok_or_else(|| {
                            Error::InvalidQuery("$not requires document".into())
                        })?;
                        if Self::matches(doc, sub_doc)? {
                            return Ok(false);
                        }
                    }
                    _ => return Err(Error::InvalidQuery(format!("unknown operator: {}", key))),
                }
            } else {
                if !Self::match_field(doc, key, value)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn match_field(doc: &Document, key: &str, value: &Bson) -> Result<bool> {
        let doc_value = doc.get(key);

        if let Bson::Document(value) = value {
            for (op, op_value) in value {
                match op.as_str() {
                    "$eq" => {
                        if !Self::values_equal(doc_value, op_value) {
                            return Ok(false);
                        }
                    }
                    "$ne" => {
                        if Self::values_equal(doc_value, op_value) {
                            return Ok(false);
                        }
                    }
                    "$gt" => {
                        if !Self::compare_values(doc_value, op_value, |cmp| cmp > 0) {
                            return Ok(false);
                        }
                    }
                    "$gte" => {
                        if !Self::compare_values(doc_value, op_value, |cmp| cmp >= 0) {
                            return Ok(false);
                        }
                    }
                    "$lt" => {
                        if !Self::compare_values(doc_value, op_value, |cmp| cmp < 0) {
                            return Ok(false);
                        }
                    }
                    "$lte" => {
                        if !Self::compare_values(doc_value, op_value, |cmp| cmp <= 0) {
                            return Ok(false);
                        }
                    }
                    "$in" => {
                        let arr = op_value.as_array().ok_or_else(|| {
                            Error::InvalidQuery("$in requires array".into())
                        })?;
                        if let Some(doc_val) = doc_value {
                            if !arr.iter().any(|v| Self::values_equal(Some(doc_val), v)) {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }
                    }
                    "$nin" => {
                        let arr = op_value.as_array().ok_or_else(|| {
                            Error::InvalidQuery("$nin requires array".into())
                        })?;
                        if let Some(doc_val) = doc_value {
                            if arr.iter().any(|v| Self::values_equal(Some(doc_val), v)) {
                                return Ok(false);
                            }
                        }
                    }
                    "$exists" => {
                        let exists = op_value.as_bool().ok_or_else(|| {
                            Error::InvalidQuery("$exists requires boolean".into())
                        })?;
                        let actual_exists = doc_value.is_some();
                        if exists != actual_exists {
                            return Ok(false);
                        }
                    }
                    "$type" => {
                        let type_str = op_value.as_str().ok_or_else(|| {
                            Error::InvalidQuery("$type requires string".into())
                        })?;
                        if let Some(doc_val) = doc_value {
                            if Self::bson_type_name(doc_val) != type_str {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }
                    }
                    "$regex" => {
                        let pattern = op_value.as_str().ok_or_else(|| {
                            Error::InvalidQuery("$regex requires string".into())
                        })?;
                        if let Some(doc_val) = doc_value {
                            if let Some(s) = doc_val.as_str() {
                                let re = regex::Regex::new(pattern).map_err(|e| {
                                    Error::InvalidQuery(format!("invalid regex: {}", e))
                                })?;
                                if !re.is_match(s) {
                                    return Ok(false);
                                }
                            } else {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }
                    }
                    "$all" => {
                        let arr = op_value.as_array().ok_or_else(|| {
                            Error::InvalidQuery("$all requires array".into())
                        })?;
                        if let Some(doc_val) = doc_value {
                            if let Some(doc_arr) = doc_val.as_array() {
                                for expected in arr {
                                    if !doc_arr.iter().any(|v| Self::values_equal(Some(v), expected)) {
                                        return Ok(false);
                                    }
                                }
                            } else {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }
                    }
                    "$size" => {
                        let expected = match op_value {
                            Bson::Int32(v) => *v as usize,
                            Bson::Int64(v) => *v as usize,
                            Bson::Double(v) => *v as usize,
                            _ => return Err(Error::InvalidQuery("$size requires number".into())),
                        };
                        if let Some(doc_val) = doc_value {
                            if let Some(doc_arr) = doc_val.as_array() {
                                if doc_arr.len() != expected {
                                    return Ok(false);
                                }
                            } else {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }
                    }
                    "$elemMatch" => {
                        let sub_doc = op_value.as_document().ok_or_else(|| {
                            Error::InvalidQuery("$elemMatch requires document".into())
                        })?;
                        if let Some(doc_val) = doc_value {
                            if let Some(doc_arr) = doc_val.as_array() {
                                let mut any_match = false;
                                for elem in doc_arr {
                                    if let Some(elem_doc) = elem.as_document() {
                                        if Self::matches(elem_doc, sub_doc)? {
                                            any_match = true;
                                            break;
                                        }
                                    }
                                }
                                if !any_match {
                                    return Ok(false);
                                }
                            } else {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }
                    }
                    "$not" => {
                        let inner = op_value.as_document().ok_or_else(|| {
                            Error::InvalidQuery("$not requires document".into())
                        })?;
                        if Self::match_field(doc, key, &Bson::Document(inner.clone()))? {
                            return Ok(false);
                        }
                    }
                    _ => return Err(Error::InvalidQuery(format!("unknown operator: {}", op))),
                }
            }
            Ok(true)
        } else {
            Ok(Self::values_equal(doc_value, value))
        }
    }

    fn values_equal(a: Option<&Bson>, b: &Bson) -> bool {
        match (a, b) {
            (Some(a), b) => a == b,
            (None, Bson::Null) => true,
            _ => false,
        }
    }

    fn compare_values<F>(a: Option<&Bson>, b: &Bson, pred: F) -> bool
    where
        F: Fn(i32) -> bool,
    {
        match (a, b) {
            (Some(a), b) => pred(Self::cmp_bson(a, b)),
            _ => false,
        }
    }

    fn cmp_bson(a: &Bson, b: &Bson) -> i32 {
        match (a, b) {
            (Bson::Int32(a), Bson::Int32(b)) => a.cmp(b) as i32,
            (Bson::Int64(a), Bson::Int64(b)) => a.cmp(b) as i32,
            (Bson::Double(a), Bson::Double(b)) => {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) as i32
            }
            (Bson::String(a), Bson::String(b)) => a.cmp(b) as i32,
            (Bson::Int32(a), Bson::Int64(b)) => (*a as i64).cmp(b) as i32,
            (Bson::Int64(a), Bson::Int32(b)) => a.cmp(&(*b as i64)) as i32,
            (Bson::Double(a), Bson::Int32(b)) => {
                a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal) as i32
            }
            (Bson::Int32(a), Bson::Double(b)) => {
                (*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) as i32
            }
            (Bson::Double(a), Bson::Int64(b)) => {
                a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal) as i32
            }
            (Bson::Int64(a), Bson::Double(b)) => {
                (*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) as i32
            }
            (Bson::ObjectId(a), Bson::ObjectId(b)) => a.bytes().cmp(&b.bytes()) as i32,
            (Bson::Boolean(a), Bson::Boolean(b)) => a.cmp(b) as i32,
            (Bson::Null, Bson::Null) => 0,
            _ => 0,
        }
    }

    fn bson_type_name(value: &Bson) -> &'static str {
        match value {
            Bson::Double(_) => "double",
            Bson::String(_) => "string",
            Bson::Array(_) => "array",
            Bson::Document(_) => "object",
            Bson::Boolean(_) => "bool",
            Bson::Null => "null",
            Bson::Int32(_) => "int",
            Bson::Int64(_) => "long",
            Bson::ObjectId(_) => "objectId",
            Bson::DateTime(_) => "date",
            Bson::Binary(_) => "binData",
            Bson::RegularExpression(_) => "regex",
            _ => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_filter() {
        let doc = bson::doc! { "name": "Alice" };
        assert!(QueryMatcher::matches(&doc, &bson::doc! {}).unwrap());
    }

    #[test]
    fn test_eq() {
        let doc = bson::doc! { "name": "Alice", "age": 30 };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "name": "Alice" }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "name": "Bob" }).unwrap());
    }

    #[test]
    fn test_ne() {
        let doc = bson::doc! { "name": "Alice" };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "name": { "$ne": "Bob" } }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "name": { "$ne": "Alice" } }).unwrap());
    }

    #[test]
    fn test_gt_lt() {
        let doc = bson::doc! { "age": 30 };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "age": { "$gt": 25 } }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "age": { "$gt": 30 } }).unwrap());
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "age": { "$gte": 30 } }).unwrap());
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "age": { "$lt": 35 } }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "age": { "$lt": 30 } }).unwrap());
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "age": { "$lte": 30 } }).unwrap());
    }

    #[test]
    fn test_in() {
        let doc = bson::doc! { "status": "active" };
        assert!(QueryMatcher::matches(
            &doc,
            &bson::doc! { "status": { "$in": ["active", "pending"] } }
        )
        .unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "status": { "$in": ["inactive"] } }).unwrap());
    }

    #[test]
    fn test_nin() {
        let doc = bson::doc! { "status": "active" };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "status": { "$nin": ["inactive"] } }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "status": { "$nin": ["active"] } }).unwrap());
    }

    #[test]
    fn test_and() {
        let doc = bson::doc! { "name": "Alice", "age": 30 };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "$and": [
            { "name": "Alice" },
            { "age": { "$gte": 25 } }
        ] }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "$and": [
            { "name": "Alice" },
            { "age": { "$gte": 35 } }
        ] }).unwrap());
    }

    #[test]
    fn test_or() {
        let doc = bson::doc! { "name": "Alice" };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "$or": [
            { "name": "Alice" },
            { "name": "Bob" }
        ] }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "$or": [
            { "name": "Charlie" },
            { "name": "Bob" }
        ] }).unwrap());
    }

    #[test]
    fn test_nor() {
        let doc = bson::doc! { "name": "Alice" };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "$nor": [
            { "name": "Bob" },
            { "name": "Charlie" }
        ] }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "$nor": [
            { "name": "Alice" },
            { "name": "Bob" }
        ] }).unwrap());
    }

    #[test]
    fn test_not() {
        let doc = bson::doc! { "name": "Alice" };
        assert!(QueryMatcher::matches(
            &doc,
            &bson::doc! { "name": { "$not": { "$eq": "Bob" } } }
        )
        .unwrap());
        assert!(!QueryMatcher::matches(
            &doc,
            &bson::doc! { "name": { "$not": { "$eq": "Alice" } } }
        )
        .unwrap());
    }

    #[test]
    fn test_exists() {
        let doc = bson::doc! { "name": "Alice" };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "name": { "$exists": true } }).unwrap());
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "email": { "$exists": false } }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "email": { "$exists": true } }).unwrap());
    }

    #[test]
    fn test_type() {
        let doc = bson::doc! { "name": "Alice", "age": 30 };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "name": { "$type": "string" } }).unwrap());
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "age": { "$type": "int" } }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "name": { "$type": "int" } }).unwrap());
    }

    #[test]
    fn test_regex() {
        let doc = bson::doc! { "name": "Alice" };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "name": { "$regex": "^Ali" } }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "name": { "$regex": "^Bob" } }).unwrap());
    }

    #[test]
    fn test_all() {
        let doc = bson::doc! { "tags": ["rust", "database", "mongodb"] };
        assert!(QueryMatcher::matches(
            &doc,
            &bson::doc! { "tags": { "$all": ["rust", "mongodb"] } }
        )
        .unwrap());
        assert!(!QueryMatcher::matches(
            &doc,
            &bson::doc! { "tags": { "$all": ["rust", "python"] } }
        )
        .unwrap());
    }

    #[test]
    fn test_size() {
        let doc = bson::doc! { "tags": ["a", "b", "c"] };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "tags": { "$size": 3 } }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "tags": { "$size": 2 } }).unwrap());
    }

    #[test]
    fn test_elem_match() {
        let doc = bson::doc! { "scores": [
            { "subject": "math", "score": 90 },
            { "subject": "english", "score": 80 }
        ] };
        assert!(QueryMatcher::matches(&doc, &bson::doc! { "scores": { "$elemMatch": { "subject": "math", "score": { "$gte": 85 } } } }).unwrap());
        assert!(!QueryMatcher::matches(&doc, &bson::doc! { "scores": { "$elemMatch": { "subject": "english", "score": { "$gte": 85 } } } }).unwrap());
    }
}
