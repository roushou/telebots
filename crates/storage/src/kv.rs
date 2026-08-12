//! Persistent key-value access: settings, cooldowns, counters.

use super::Storage;
use crate::Error;

impl Storage {
    /// Store `value` under `key`, replacing any previous value.
    pub async fn kv_set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        let key = key.to_owned();
        let value = value.to_owned();
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT INTO kv (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )?;
            Ok(())
        })
        .await
    }

    /// Fetch the value stored under `key`, if any.
    pub async fn kv_get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let key = key.to_owned();
        self.with_conn(move |conn| {
            let mut stmt = conn.prepare("SELECT value FROM kv WHERE key = ?1")?;
            let mut rows = stmt.query(rusqlite::params![key])?;
            Ok(rows.next()?.map(|row| row.get(0)).transpose()?)
        })
        .await
    }

    /// Remove the value stored under `key`.
    pub async fn kv_delete(&self, key: &str) -> Result<(), Error> {
        let key = key.to_owned();
        self.with_conn(move |conn| {
            conn.execute("DELETE FROM kv WHERE key = ?1", rusqlite::params![key])?;
            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, Storage};

    #[tokio::test]
    async fn kv_roundtrip() -> Result<(), Error> {
        let store = Storage::open(":memory:").await?;
        assert!(store.kv_get("k").await?.is_none());
        store.kv_set("k", b"v1").await?;
        assert_eq!(store.kv_get("k").await?, Some(b"v1".to_vec()));
        store.kv_set("k", b"v2").await?;
        assert_eq!(store.kv_get("k").await?, Some(b"v2".to_vec()));
        store.kv_delete("k").await?;
        assert!(store.kv_get("k").await?.is_none());
        Ok(())
    }
}
