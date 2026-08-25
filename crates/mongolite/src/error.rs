use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("BSON error: {0}")]
    Bson(String),

    #[error("Invalid page size: {0}")]
    InvalidPageSize(u32),

    #[error("Corrupted database: {0}")]
    Corrupted(String),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Duplicate key: {0}")]
    DuplicateKey(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Invalid update: {0}")]
    InvalidUpdate(String),

    #[error("Database is locked")]
    Locked,

    #[error("WAL error: {0}")]
    Wal(String),
}

pub type Result<T> = std::result::Result<T, Error>;
