//! Remind's database: the reminders and the per-chat timezone offset.

use anyhow::Result;
use storage::{Migration, Storage, Value};

/// Remind's schema, as one migration.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: SCHEMA,
}];

const SCHEMA: &str = "
    CREATE TABLE reminders (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        chat_id    INTEGER NOT NULL,
        user_id    INTEGER,
        fire_at    INTEGER NOT NULL,
        message    TEXT NOT NULL,
        created_at INTEGER NOT NULL
    );
    CREATE INDEX reminders_fire_at_idx ON reminders(fire_at);
    CREATE TABLE settings (
        chat_id        INTEGER PRIMARY KEY,
        utc_offset_min INTEGER NOT NULL DEFAULT 0
    );";

/// One stored reminder.
#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: i64,
    pub chat_id: i64,
    pub fire_at: i64,
    pub message: String,
}

/// How many reminders `/reminders` lists at most.
const LIST_LIMIT: usize = 20;

/// Remind's typed store over the shared connection.
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

    /// Record a reminder and return nothing (the list shows the id).
    pub async fn add_reminder(
        &self,
        chat_id: i64,
        user_id: Option<i64>,
        fire_at: i64,
        message: &str,
    ) -> Result<()> {
        let created_at = telebots_core::Time::now_secs();
        self.storage
            .execute(
                "INSERT INTO reminders (chat_id, user_id, fire_at, message, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                &[
                    Value::Integer(chat_id),
                    user_id.map_or(Value::Null, Value::Integer),
                    Value::Integer(fire_at),
                    Value::Text(message.to_owned()),
                    Value::Integer(created_at),
                ],
            )
            .await?;
        Ok(())
    }

    /// The pending reminders for a chat, soonest first.
    pub async fn list_reminders(&self, chat_id: i64) -> Result<Vec<Reminder>> {
        Ok(self
            .storage
            .query(
                "SELECT id, chat_id, fire_at, message FROM reminders
                 WHERE chat_id = ?1 ORDER BY fire_at ASC, id ASC LIMIT ?2",
                &[Value::Integer(chat_id), Value::Integer(LIST_LIMIT as i64)],
                |row| {
                    Ok(Reminder {
                        id: row.get(0)?,
                        chat_id: row.get(1)?,
                        fire_at: row.get(2)?,
                        message: row.get(3)?,
                    })
                },
            )
            .await?)
    }

    /// Delete one of the chat's reminders; `true` when it existed.
    pub async fn cancel_reminder(&self, chat_id: i64, id: i64) -> Result<bool> {
        let removed = self
            .storage
            .execute(
                "DELETE FROM reminders WHERE id = ?1 AND chat_id = ?2",
                &[Value::Integer(id), Value::Integer(chat_id)],
            )
            .await?;
        Ok(removed > 0)
    }

    /// Every reminder due at or before `now`, across all chats.
    pub async fn due_reminders(&self, now: i64) -> Result<Vec<Reminder>> {
        Ok(self
            .storage
            .query(
                "SELECT id, chat_id, fire_at, message FROM reminders
                 WHERE fire_at <= ?1 ORDER BY fire_at ASC, id ASC",
                &[Value::Integer(now)],
                |row| {
                    Ok(Reminder {
                        id: row.get(0)?,
                        chat_id: row.get(1)?,
                        fire_at: row.get(2)?,
                        message: row.get(3)?,
                    })
                },
            )
            .await?)
    }

    /// Delete a delivered reminder.
    pub async fn delete_reminder(&self, id: i64) -> Result<()> {
        self.storage
            .execute("DELETE FROM reminders WHERE id = ?1", &[Value::Integer(id)])
            .await?;
        Ok(())
    }

    /// A chat's UTC offset in minutes (default `0` = UTC).
    pub async fn utc_offset(&self, chat_id: i64) -> Result<i16> {
        let rows: Vec<i64> = self
            .storage
            .query(
                "SELECT utc_offset_min FROM settings WHERE chat_id = ?1",
                &[Value::Integer(chat_id)],
                |row| row.get(0),
            )
            .await?;
        Ok(rows.into_iter().next().unwrap_or(0) as i16)
    }

    /// Persist a chat's UTC offset, upserting on chat id.
    pub async fn set_utc_offset(&self, chat_id: i64, minutes: i16) -> Result<()> {
        self.storage
            .execute(
                "INSERT INTO settings (chat_id, utc_offset_min) VALUES (?1, ?2)
                 ON CONFLICT(chat_id) DO UPDATE SET utc_offset_min = excluded.utc_offset_min",
                &[Value::Integer(chat_id), Value::Integer(i64::from(minutes))],
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn store() -> Store {
        Store::open(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn reminders_list_soonest_first() -> Result<()> {
        let store = store().await;
        store.add_reminder(1, Some(42), 300, "later").await?;
        store.add_reminder(1, Some(42), 100, "first").await?;
        let list = store.list_reminders(1).await?;
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].message, "first");
        assert_eq!(list[1].message, "later");
        Ok(())
    }

    #[tokio::test]
    async fn list_is_scoped_to_chat() -> Result<()> {
        let store = store().await;
        store.add_reminder(1, Some(42), 100, "mine").await?;
        store.add_reminder(2, Some(42), 100, "theirs").await?;
        assert_eq!(store.list_reminders(1).await?.len(), 1);
        assert_eq!(store.list_reminders(2).await?.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn cancel_is_scoped_to_chat() -> Result<()> {
        let store = store().await;
        store.add_reminder(1, Some(42), 100, "mine").await?;
        let id = store.list_reminders(1).await?[0].id;
        // A different chat cannot cancel it.
        assert!(!store.cancel_reminder(2, id).await?);
        assert!(store.cancel_reminder(1, id).await?);
        assert!(store.list_reminders(1).await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn due_returns_only_overdue() -> Result<()> {
        let store = store().await;
        store.add_reminder(1, Some(42), 100, "due").await?;
        store.add_reminder(2, Some(42), 200, "later").await?;
        let due = store.due_reminders(150).await?;
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].message, "due");
        Ok(())
    }

    #[tokio::test]
    async fn delete_removes_a_reminder() -> Result<()> {
        let store = store().await;
        store.add_reminder(1, Some(42), 100, "x").await?;
        let id = store.list_reminders(1).await?[0].id;
        store.delete_reminder(id).await?;
        assert!(store.list_reminders(1).await?.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn utc_offset_defaults_and_roundtrips() -> Result<()> {
        let store = store().await;
        assert_eq!(store.utc_offset(1).await?, 0);
        store.set_utc_offset(1, 330).await?;
        assert_eq!(store.utc_offset(1).await?, 330);
        store.set_utc_offset(1, -300).await?;
        assert_eq!(store.utc_offset(1).await?, -300);
        assert_eq!(store.utc_offset(2).await?, 0);
        Ok(())
    }
}
