use std::ffi::{CStr, c_char, c_int};
use std::ptr;

use mongolite::db::Database;

#[no_mangle]
pub extern "C" fn mongolite_create(path: *const c_char) -> *mut Database {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match Database::create(path_str) {
        Ok(db) => Box::leak(Box::new(db)),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn mongolite_open(path: *const c_char) -> *mut Database {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match Database::open(path_str) {
        Ok(db) => Box::leak(Box::new(db)),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn mongolite_close(db: *mut Database) -> c_int {
    if db.is_null() {
        return -1;
    }
    unsafe {
        let _ = (*db).flush();
        drop(Box::from_raw(db));
    }
    0
}

#[no_mangle]
pub extern "C" fn mongolite_insert(
    db: *mut Database,
    collection: *const c_char,
    json: *const c_char,
    out_id: *mut c_char,
    out_id_len: c_int,
) -> c_int {
    unsafe {
        if db.is_null() || collection.is_null() || json.is_null() {
            return -1;
        }

        let coll_str = match CStr::from_ptr(collection).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };

        let json_str = match CStr::from_ptr(json).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };

        let doc: bson::Document = match serde_json::from_str(json_str) {
            Ok(d) => d,
            Err(_) => return -1,
        };

        let mut coll = (*db).collection(coll_str);
        match coll.insert_one(doc) {
            Ok(result) => {
                let id_hex = result.inserted_id.to_hex();
                if !out_id.is_null() && out_id_len >= 25 {
                    ptr::copy_nonoverlapping(id_hex.as_ptr() as *const c_char, out_id, 24);
                    *out_id.add(24) = 0;
                }
                0
            }
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn mongolite_find(
    db: *mut Database,
    collection: *const c_char,
    filter: *const c_char,
    out_json: *mut c_char,
    out_json_len: c_int,
) -> c_int {
    unsafe {
        if db.is_null() || collection.is_null() {
            return -1;
        }

        let coll_str = match CStr::from_ptr(collection).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };

        let filter_doc = if filter.is_null() {
            None
        } else {
            let filter_str = match CStr::from_ptr(filter).to_str() {
                Ok(s) => s,
                Err(_) => return -1,
            };
            match serde_json::from_str::<bson::Document>(filter_str) {
                Ok(d) => Some(d),
                Err(_) => return -1,
            }
        };

        let coll = (*db).collection(coll_str);
        match coll.find(filter_doc) {
            Ok(docs) => {
                let json_arr: Vec<String> = docs
                    .iter()
                    .map(|d| serde_json::to_string(d).unwrap_or_default())
                    .collect();
                let result = format!("[{}]", json_arr.join(","));
                if !out_json.is_null() && out_json_len > 0 {
                    let copy_len = std::cmp::min(result.len(), out_json_len as usize - 1);
                    ptr::copy_nonoverlapping(result.as_ptr() as *const c_char, out_json, copy_len);
                    *out_json.add(copy_len) = 0;
                }
                result.len() as c_int
            }
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn mongolite_count(
    db: *mut Database,
    collection: *const c_char,
) -> c_int {
    unsafe {
        if db.is_null() || collection.is_null() {
            return -1;
        }

        let coll_str = match CStr::from_ptr(collection).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };

        let coll = (*db).collection(coll_str);
        match coll.count(None) {
            Ok(count) => count as c_int,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn mongolite_delete(
    db: *mut Database,
    collection: *const c_char,
    filter: *const c_char,
) -> c_int {
    unsafe {
        if db.is_null() || collection.is_null() || filter.is_null() {
            return -1;
        }

        let coll_str = match CStr::from_ptr(collection).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };

        let filter_str = match CStr::from_ptr(filter).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };

        let filter_doc: bson::Document = match serde_json::from_str(filter_str) {
            Ok(d) => d,
            Err(_) => return -1,
        };

        let mut coll = (*db).collection(coll_str);
        match coll.delete_many(filter_doc) {
            Ok(result) => result.deleted_count as c_int,
            Err(_) => -1,
        }
    }
}
