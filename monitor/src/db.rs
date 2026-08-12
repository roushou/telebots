//! Snapshot storage: the monitor's `snapshots` table on top of the shared
//! [`storage`] connection.

use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use serde_json::Value;
use storage::{Storage, rusqlite::types::Value as SqlValue};

/// One status snapshot for a bot.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub bot: String,
    pub ts: i64,
    /// The bot's `/metrics` JSON when reachable.
    pub status: Option<Value>,
    /// Why the bot was unreachable.
    pub error: Option<String>,
}

const SNAPSHOTS_DDL: &str = "
    CREATE TABLE IF NOT EXISTS snapshots (
        bot    TEXT NOT NULL,
        ts     INTEGER NOT NULL,
        status TEXT,
        error  TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_snapshots_bot_ts
        ON snapshots (bot, ts);";

/// Snapshot store; cheap to clone, one connection.
#[derive(Clone)]
pub struct Db {
    storage: Storage,
}

impl Db {
    /// Open (creating if missing) the database at `path` and ensure the
    /// snapshots schema exists.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_owned();
        let storage = Storage::open(&path).await?;
        storage.execute_batch(SNAPSHOTS_DDL).await?;
        Ok(Self { storage })
    }

    /// Append one snapshot.
    pub async fn insert_snapshot(
        &self,
        bot: &str,
        status: Option<&Value>,
        error: Option<&str>,
    ) -> Result<()> {
        self.storage
            .execute(
                "INSERT INTO snapshots (bot, ts, status, error) VALUES (?1, ?2, ?3, ?4)",
                &[
                    SqlValue::Text(bot.to_string()),
                    SqlValue::Integer(Self::now_unix()),
                    status.map_or(SqlValue::Null, |s| SqlValue::Text(s.to_string())),
                    error.map_or(SqlValue::Null, |e| SqlValue::Text(e.to_string())),
                ],
            )
            .await?;
        Ok(())
    }

    /// The newest snapshot per bot.
    pub async fn latest_per_bot(&self) -> Result<Vec<Snapshot>> {
        Ok(self
            .storage
            .query(
                "SELECT bot, ts, status, error FROM snapshots s
                 WHERE ts = (SELECT MAX(ts) FROM snapshots WHERE bot = s.bot)",
                &[],
                Self::row_to_snapshot,
            )
            .await?)
    }

    /// The newest `limit` snapshots for one bot, newest first.
    pub async fn history(&self, bot: &str, limit: usize) -> Result<Vec<Snapshot>> {
        Ok(self
            .storage
            .query(
                "SELECT bot, ts, status, error FROM snapshots
                 WHERE bot = ?1 ORDER BY ts DESC LIMIT ?2",
                &[
                    SqlValue::Text(bot.to_string()),
                    SqlValue::Integer(limit as i64),
                ],
                Self::row_to_snapshot,
            )
            .await?)
    }

    fn row_to_snapshot(row: &storage::rusqlite::Row<'_>) -> storage::rusqlite::Result<Snapshot> {
        Ok(Snapshot {
            bot: row.get(0)?,
            ts: row.get(1)?,
            status: row
                .get::<_, Option<String>>(2)?
                .and_then(|s| serde_json::from_str(&s).ok()),
            error: row.get(3)?,
        })
    }

    fn now_unix() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}
