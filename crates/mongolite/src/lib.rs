pub mod db;
pub mod document;
pub mod error;
pub mod storage;

pub use db::Database;
pub use error::{Error, Result};
