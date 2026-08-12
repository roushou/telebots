//! Append-only record log: history of user actions.

use std::time::{SystemTime, UNIX_EPOCH};

use super::Storage;
use crate::Error;

/// One entry in the append-only log.
#[derive(Debug, Clone)]
pub struct Record {
    /// Id assigned by the store; `None` when appending.
    pub id: Option<u64>,
    pub chat_id: i64,
    pub user_id: Option<i64>,
    /// Namespace for the entry, e.g. `"image"`; `recent` filters by it.
    pub kind: String,
    /// Free-form text, e.g. a user prompt.
    pub text: Option<String>,
    /// Opaque payload, e.g. generated image bytes.
    pub payload: Option<Vec<u8>>,
    /// Unix seconds; `None` means now.
    pub created_at: Option<i64>,
}

impl Storage {
    /// Append a record to the log; returns its id. `created_at` defaults to
    /// now when `None`.
    pub async fn append(&self, record: Record) -> Result<u64, Error> {
        let created_at = record.created_at.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        });
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT INTO records (chat_id, user_id, kind, text, payload, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    record.chat_id,
                    record.user_id,
                    record.kind,
                    record.text,
                    record.payload,
                    created_at
                ],
            )?;
            Ok(conn.last_insert_rowid() as u64)
        })
        .await
    }

    /// The `limit` most recent records for a chat and kind, newest first.
    pub async fn recent(
        &self,
        chat_id: i64,
        kind: &str,
        limit: usize,
    ) -> Result<Vec<Record>, Error> {
        let kind = kind.to_owned();
        self.with_conn(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, chat_id, user_id, kind, text, payload, created_at
                 FROM records
                 WHERE chat_id = ?1 AND kind = ?2
                 ORDER BY id DESC
                 LIMIT ?3",
            )?;
            let rows = stmt.query_map(rusqlite::params![chat_id, kind, limit as i64], |row| {
                Ok(Record {
                    id: Some(row.get::<_, i64>(0)? as u64),
                    chat_id: row.get(1)?,
                    user_id: row.get(2)?,
                    kind: row.get(3)?,
                    text: row.get(4)?,
                    payload: row.get(5)?,
                    created_at: Some(row.get(6)?),
                })
            })?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Storage;

    fn record(chat_id: i64, kind: &str, text: &str) -> Record {
        Record {
            id: None,
            chat_id,
            user_id: Some(42),
            kind: kind.to_string(),
            text: Some(text.into()),
            payload: None,
            created_at: None,
        }
    }

    #[tokio::test]
    async fn records_are_newest_first_and_filtered() -> Result<(), Error> {
        let store = Storage::open(":memory:").await?;
        store.append(record(1, "image", "a cat")).await?;
        store.append(record(1, "image", "a dog")).await?;
        store.append(record(1, "other", "ignored")).await?;
        store.append(record(2, "image", "other chat")).await?;

        let recent = store.recent(1, "image", 10).await?;
        let texts: Vec<String> = recent.iter().filter_map(|r| r.text.clone()).collect();
        assert_eq!(texts, ["a dog", "a cat"]);
        assert!(recent[0].id.is_some());

        let limited = store.recent(1, "image", 1).await?;
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].text.as_deref(), Some("a dog"));
        Ok(())
    }
}
