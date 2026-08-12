//! Reusable SQLite storage for Telebots bots.
//!
//! A bot plugs in its own database file and gets a small async API over a
//! single connection: a persistent key-value store (settings, cooldowns,
//! counters) and an append-only record log (history of user actions).
//!
//! Organized by concern: [`kv`] and [`records`] provide the two stores;
//! [`Storage`] owns the connection. Queries run on a blocking thread; the
//! API is fully async.

mod error;
mod kv;
mod records;
mod sql;

use std::{path::Path, sync::Arc};

pub use error::Error;
pub use records::Record;
pub use rusqlite;
use rusqlite::Connection;
use tokio::sync::Mutex;

/// A SQLite-backed store. Cheap to clone; all instances share one connection.
#[derive(Clone)]
pub struct Storage {
    inner: Arc<Mutex<Connection>>,
}

impl Storage {
    /// Open (creating if missing) the database at `path` and ensure the
    /// schema exists. Pass `:memory:` for an in-memory database.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_owned();
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&path).map_err(|source| Error::Open {
                path: path.clone(),
                source,
            })?;
            create_schema(&conn)?;
            Ok(Self {
                inner: Arc::new(Mutex::new(conn)),
            })
        })
        .await?
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

fn create_schema(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA busy_timeout=5000;
         PRAGMA foreign_keys=ON;
         CREATE TABLE IF NOT EXISTS kv (
             key   TEXT PRIMARY KEY,
             value BLOB NOT NULL
         );
         CREATE TABLE IF NOT EXISTS records (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             chat_id    INTEGER NOT NULL,
             user_id    INTEGER,
             kind       TEXT NOT NULL,
             text       TEXT,
             payload    BLOB,
             created_at INTEGER NOT NULL
         );",
    )?;
    Ok(())
}
