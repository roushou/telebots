//! Reusable SQLite storage for Telebots bots.
//!
//! A bot opens its own database file and gets a small async API over a
//! single connection: the generic `execute`/`query` interface plus versioned
//! schema migrations. Consumers define their own tables and typed accessors
//! on top; this crate owns the connection and the migration runner.

mod error;
mod migrations;
mod sql;

use std::{path::Path, sync::Arc};

pub use error::Error;
pub use migrations::Migration;
use rusqlite::Connection;
pub use rusqlite::{self, types::Value};
use tokio::sync::Mutex;

/// A SQLite-backed store. Cheap to clone; all instances share one connection.
#[derive(Clone)]
pub struct Storage {
    inner: Arc<Mutex<Connection>>,
}

impl Storage {
    /// Open (creating if missing) the database at `path` and configure the
    /// connection. Pass `:memory:` for an in-memory database.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_owned();
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&path).map_err(|source| Error::Open {
                path: path.clone(),
                source,
            })?;
            migrations::configure(&conn)?;
            Ok(Self {
                inner: Arc::new(Mutex::new(conn)),
            })
        })
        .await?
    }

    /// Apply any migrations newer than the current schema version, in order.
    pub async fn migrate(&self, migrations: &[Migration]) -> Result<(), Error> {
        let migrations = migrations.to_vec();
        self.with_conn(move |conn| migrations::run(conn, &migrations))
            .await
    }

    /// Run `f` against the connection on a blocking thread.
    async fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error>
    where
        T: Send + 'static,
    {
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            let conn = inner.blocking_lock();
            f(&conn)
        })
        .await?
    }
}
