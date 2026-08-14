//! Imagine's database: the per-user cooldown and the generation history.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use storage::{Migration, Storage, Value};

/// Imagine's schema, as one migration.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: SCHEMA,
}];

const SCHEMA: &str = "
    CREATE TABLE cooldowns (
        chat_id      INTEGER NOT NULL,
        user_id      INTEGER NOT NULL,
        last_used_at INTEGER NOT NULL,
        PRIMARY KEY (chat_id, user_id)
    );
    CREATE TABLE generations (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        chat_id    INTEGER NOT NULL,
        user_id    INTEGER,
        prompt     TEXT NOT NULL,
        model      TEXT NOT NULL,
        jpeg       BLOB,
        created_at INTEGER NOT NULL
    );";

/// One recorded generation.
#[derive(Debug, Clone)]
pub struct Generation {
    pub id: i64,
    pub prompt: String,
    pub model: String,
}

/// Imagine's typed store over the shared connection.
#[derive(Clone)]
pub struct Store {
    storage: Storage,
}

impl Store {
    /// Open and migrate the database.
    pub async fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let storage = Storage::open(path).await?;
        storage.migrate(MIGRATIONS).await?;
        Ok(Self { storage })
    }

    /// The last-used timestamp for a user's cooldown, when any.
    pub async fn cooldown(&self, chat_id: i64, user_id: i64) -> Result<Option<i64>> {
        let rows: Vec<i64> = self
            .storage
            .query(
                "SELECT last_used_at FROM cooldowns WHERE chat_id = ?1 AND user_id = ?2",
                &[Value::Integer(chat_id), Value::Integer(user_id)],
                |row| row.get(0),
            )
            .await?;
        Ok(rows.into_iter().next())
    }

    /// Record a cooldown use, upserting on (chat, user).
    pub async fn set_cooldown(&self, chat_id: i64, user_id: i64, at: i64) -> Result<()> {
        self.storage
            .execute(
                "INSERT INTO cooldowns (chat_id, user_id, last_used_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(chat_id, user_id) DO UPDATE SET last_used_at = excluded.last_used_at",
                &[
                    Value::Integer(chat_id),
                    Value::Integer(user_id),
                    Value::Integer(at),
                ],
            )
            .await?;
        Ok(())
    }

    /// Append a generation.
    pub async fn add_generation(
        &self,
        chat_id: i64,
        user_id: Option<i64>,
        prompt: &str,
        model: &str,
        jpeg: Option<&[u8]>,
    ) -> Result<()> {
        let created_at = Self::now_secs();
        self.storage
            .execute(
                "INSERT INTO generations (chat_id, user_id, prompt, model, jpeg, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                &[
                    Value::Integer(chat_id),
                    user_id.map_or(Value::Null, Value::Integer),
                    Value::Text(prompt.to_owned()),
                    Value::Text(model.to_owned()),
                    jpeg.map_or(Value::Null, |bytes| Value::Blob(bytes.to_vec())),
                    Value::Integer(created_at),
                ],
            )
            .await?;
        Ok(())
    }

    /// The most recent generations for a chat, newest first.
    pub async fn recent_generations(&self, chat_id: i64, limit: usize) -> Result<Vec<Generation>> {
        Ok(self
            .storage
            .query(
                "SELECT id, prompt, model FROM generations
                 WHERE chat_id = ?1 ORDER BY id DESC LIMIT ?2",
                &[Value::Integer(chat_id), Value::Integer(limit as i64)],
                |row| {
                    Ok(Generation {
                        id: row.get(0)?,
                        prompt: row.get(1)?,
                        model: row.get(2)?,
                    })
                },
            )
            .await?)
    }

    fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}
