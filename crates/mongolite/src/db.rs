use std::path::Path;

use crate::error::Result;

pub struct Database;

impl Database {
    pub fn open<P: AsRef<Path>>(_path: P) -> Result<Self> {
        Ok(Self)
    }
}
