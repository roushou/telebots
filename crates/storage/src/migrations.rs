//! Versioned schema migrations, tracked via SQLite's `user_version` pragma.

use rusqlite::Connection;

use crate::Error;

/// One schema migration. Append new entries as the schema evolves; never
/// edit an entry that has shipped.
struct Migration {
    version: i64,
    sql: &'static str,
}

/// The initial schema: the persistent key-value store and the append-only
/// record log.
const INITIAL_SCHEMA: &str = "
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
    );";

/// Ordered migrations, oldest first.
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: INITIAL_SCHEMA,
}];

/// Configure the connection and apply any pending migrations.
pub fn migrate(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA busy_timeout=5000;
         PRAGMA foreign_keys=ON;",
    )?;

    let current = user_version(conn)?;
    for migration in MIGRATIONS {
        if migration.version > current {
            conn.execute_batch(migration.sql)?;
            set_user_version(conn, migration.version)?;
        }
    }
    Ok(())
}

fn user_version(conn: &Connection) -> Result<i64, Error> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

fn set_user_version(conn: &Connection, version: i64) -> Result<(), Error> {
    conn.pragma_update(None, "user_version", version)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Storage;

    #[tokio::test]
    async fn migrate_sets_version_and_schema() -> Result<(), Error> {
        let store = Storage::open(":memory:").await?;

        let version: Vec<i64> = store
            .query("PRAGMA user_version", &[], |row| row.get(0))
            .await?;
        assert_eq!(version, vec![MIGRATIONS.last().unwrap().version]);

        // The initial tables exist.
        store
            .execute("INSERT INTO kv (key, value) VALUES ('a', X'01')", &[])
            .await?;
        Ok(())
    }
}
