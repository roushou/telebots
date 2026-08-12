//! The crate's error type.

use std::path::PathBuf;

use thiserror::Error;

/// Errors returned by the storage layer.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Opening the database file failed (missing, permissions, …).
    #[error("failed to open database at {path}")]
    Open {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    /// A SQLite statement, query, or row mapping failed.
    #[error("sqlite error")]
    Sql(#[from] rusqlite::Error),

    /// The blocking task that ran the query failed to join.
    #[error("database task failed")]
    TaskJoin(#[from] tokio::task::JoinError),
}
