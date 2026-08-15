//! Bud's database: the conversation history, per-chat settings, and the
//! chat cooldown.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use cloudflare_ai::TextModel;
use storage::{Migration, Storage, Value};

/// Bud's schema, as one migration.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: SCHEMA,
}];

const SCHEMA: &str = "
    CREATE TABLE messages (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        chat_id    INTEGER NOT NULL,
        user_id    INTEGER,
        role       TEXT NOT NULL,
        content    TEXT NOT NULL,
        created_at INTEGER NOT NULL
    );
    CREATE INDEX messages_chat_id_idx ON messages(chat_id, id);
    CREATE TABLE settings (
        chat_id       INTEGER PRIMARY KEY,
        model         TEXT NOT NULL,
        system_prompt TEXT
    );
    CREATE TABLE cooldowns (
        chat_id      INTEGER NOT NULL,
        user_id      INTEGER NOT NULL,
        last_used_at INTEGER NOT NULL,
        PRIMARY KEY (chat_id, user_id)
    );";

/// One message read back from history.
#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub role: String,
    pub content: String,
}

/// A chat's persistent preferences.
#[derive(Debug, Clone)]
pub struct Settings {
    pub model: TextModel,
    pub system_prompt: Option<String>,
}

/// Bud's typed store over the shared connection.
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

    /// Append one conversation message.
    pub async fn add_message(
        &self,
        chat_id: i64,
        user_id: Option<i64>,
        role: &str,
        content: &str,
    ) -> Result<()> {
        let created_at = Self::now_secs();
        self.storage
            .execute(
                "INSERT INTO messages (chat_id, user_id, role, content, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                &[
                    Value::Integer(chat_id),
                    user_id.map_or(Value::Null, Value::Integer),
                    Value::Text(role.to_owned()),
                    Value::Text(content.to_owned()),
                    Value::Integer(created_at),
                ],
            )
            .await?;
        Ok(())
    }

    /// The most recent messages for a chat, oldest first.
    pub async fn recent_messages(&self, chat_id: i64, limit: usize) -> Result<Vec<StoredMessage>> {
        let mut rows: Vec<StoredMessage> = self
            .storage
            .query(
                "SELECT role, content FROM messages
                 WHERE chat_id = ?1 ORDER BY id DESC LIMIT ?2",
                &[Value::Integer(chat_id), Value::Integer(limit as i64)],
                |row| {
                    Ok(StoredMessage {
                        role: row.get(0)?,
                        content: row.get(1)?,
                    })
                },
            )
            .await?;
        rows.reverse();
        Ok(rows)
    }

    /// Delete all messages for a chat.
    pub async fn clear_chat(&self, chat_id: i64) -> Result<()> {
        self.storage
            .execute(
                "DELETE FROM messages WHERE chat_id = ?1",
                &[Value::Integer(chat_id)],
            )
            .await?;
        Ok(())
    }

    /// A chat's settings, defaulting the model when no row exists yet.
    pub async fn settings(&self, chat_id: i64) -> Result<Settings> {
        let rows: Vec<(String, Option<String>)> = self
            .storage
            .query(
                "SELECT model, system_prompt FROM settings WHERE chat_id = ?1",
                &[Value::Integer(chat_id)],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .await?;
        match rows.into_iter().next() {
            Some((model, system_prompt)) => Ok(Settings {
                model: model.parse().unwrap_or_else(|_| TextModel::default()),
                system_prompt,
            }),
            None => Ok(Settings {
                model: TextModel::default(),
                system_prompt: None,
            }),
        }
    }

    /// Persist a chat's model choice, creating the row if needed.
    pub async fn set_model(&self, chat_id: i64, model: TextModel) -> Result<()> {
        self.storage
            .execute(
                "INSERT INTO settings (chat_id, model) VALUES (?1, ?2)
                 ON CONFLICT(chat_id) DO UPDATE SET model = excluded.model",
                &[Value::Integer(chat_id), Value::Text(model.to_string())],
            )
            .await?;
        Ok(())
    }

    /// Persist a chat's system prompt, preserving its model choice.
    pub async fn set_system_prompt(&self, chat_id: i64, prompt: &str) -> Result<()> {
        let model = self.settings(chat_id).await?.model;
        self.storage
            .execute(
                "INSERT INTO settings (chat_id, model, system_prompt) VALUES (?1, ?2, ?3)
                 ON CONFLICT(chat_id) DO UPDATE SET system_prompt = excluded.system_prompt",
                &[
                    Value::Integer(chat_id),
                    Value::Text(model.to_string()),
                    Value::Text(prompt.to_owned()),
                ],
            )
            .await?;
        Ok(())
    }

    /// Clear a chat's system prompt (falls back to the configured default).
    pub async fn clear_system_prompt(&self, chat_id: i64) -> Result<()> {
        self.storage
            .execute(
                "UPDATE settings SET system_prompt = NULL WHERE chat_id = ?1",
                &[Value::Integer(chat_id)],
            )
            .await?;
        Ok(())
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

    fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn store() -> Store {
        Store::open(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn messages_roundtrip_oldest_first() -> Result<()> {
        let store = store().await;
        store.add_message(1, Some(42), "user", "hi").await?;
        store.add_message(1, Some(42), "assistant", "hello").await?;
        let rows = store.recent_messages(1, 10).await?;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].content, "hi");
        assert_eq!(rows[1].content, "hello");
        Ok(())
    }

    #[tokio::test]
    async fn recent_messages_respect_limit() -> Result<()> {
        let store = store().await;
        for i in 0..5 {
            store
                .add_message(1, Some(42), "user", &format!("m{i}"))
                .await?;
        }
        let rows = store.recent_messages(1, 3).await?;
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].content, "m2");
        assert_eq!(rows[2].content, "m4");
        Ok(())
    }

    #[tokio::test]
    async fn clear_chat_removes_messages() -> Result<()> {
        let store = store().await;
        store.add_message(1, Some(42), "user", "hi").await?;
        store.clear_chat(1).await?;
        assert!(store.recent_messages(1, 10).await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn settings_default_model() -> Result<()> {
        let store = store().await;
        let settings = store.settings(1).await?;
        assert_eq!(settings.model, TextModel::default());
        assert_eq!(settings.system_prompt, None);
        Ok(())
    }

    #[tokio::test]
    async fn set_model_and_system_prompt_roundtrip() -> Result<()> {
        let store = store().await;
        store.set_model(1, TextModel::Llama3370b).await?;
        store.set_system_prompt(1, "be terse").await?;
        let settings = store.settings(1).await?;
        assert_eq!(settings.model, TextModel::Llama3370b);
        assert_eq!(settings.system_prompt.as_deref(), Some("be terse"));

        // Setting a system prompt preserves the model.
        store.set_model(1, TextModel::Llama323b).await?;
        let settings = store.settings(1).await?;
        assert_eq!(settings.model, TextModel::Llama323b);
        assert_eq!(settings.system_prompt.as_deref(), Some("be terse"));
        Ok(())
    }

    #[tokio::test]
    async fn clear_system_prompt_removes_override() -> Result<()> {
        let store = store().await;
        store.set_system_prompt(1, "be terse").await?;
        store.clear_system_prompt(1).await?;
        assert_eq!(store.settings(1).await?.system_prompt, None);
        Ok(())
    }

    #[tokio::test]
    async fn cooldown_upserts() -> Result<()> {
        let store = store().await;
        assert_eq!(store.cooldown(1, 42).await?, None);
        store.set_cooldown(1, 42, 100).await?;
        store.set_cooldown(1, 42, 200).await?;
        assert_eq!(store.cooldown(1, 42).await?, Some(200));
        assert_eq!(store.cooldown(1, 7).await?, None);
        Ok(())
    }
}
