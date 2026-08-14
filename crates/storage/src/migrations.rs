//! Versioned schema migrations, tracked via SQLite's `user_version` pragma.

use rusqlite::Connection;

use crate::Error;

/// One schema migration, applied once when the database is behind it.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: i64,
    pub sql: &'static str,
}

/// Configure the connection (run on every open).
pub(crate) fn configure(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA busy_timeout=5000;
         PRAGMA foreign_keys=ON;",
    )?;
    Ok(())
}

/// Apply any migrations newer than the current `user_version`, in order.
pub(crate) fn run(conn: &Connection, migrations: &[Migration]) -> Result<(), Error> {
    let current = user_version(conn)?;
    for migration in migrations {
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

    const TEST_MIGRATIONS: &[Migration] = &[
        Migration {
            version: 1,
            sql: "CREATE TABLE a (x INTEGER);",
        },
        Migration {
            version: 2,
            sql: "CREATE TABLE b (y INTEGER);",
        },
    ];

    #[tokio::test]
    async fn migrate_applies_pending_and_stamps_version() -> Result<(), Error> {
        let store = Storage::open(":memory:").await?;
        store.migrate(TEST_MIGRATIONS).await?;

        let version: Vec<i64> = store
            .query("PRAGMA user_version", &[], |row| row.get(0))
            .await?;
        assert_eq!(version, vec![2]);

        // Both migrations' tables exist.
        store.execute("INSERT INTO a (x) VALUES (1)", &[]).await?;
        store.execute("INSERT INTO b (y) VALUES (2)", &[]).await?;
        Ok(())
    }

    #[tokio::test]
    async fn migrate_skips_already_applied() -> Result<(), Error> {
        let store = Storage::open(":memory:").await?;
        store.migrate(TEST_MIGRATIONS).await?;
        // Re-running is a no-op.
        store.migrate(TEST_MIGRATIONS).await?;
        let version: Vec<i64> = store
            .query("PRAGMA user_version", &[], |row| row.get(0))
            .await?;
        assert_eq!(version, vec![2]);
        Ok(())
    }
}
