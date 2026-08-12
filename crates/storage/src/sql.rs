//! Generic SQL access: arbitrary statements and row queries for callers
//! with their own tables (the monitor's snapshots, future migrations).

use rusqlite::{ToSql, types::Value};

use super::Storage;
use crate::Error;

impl Storage {
    /// Run a batch of statements (DDL). Use for `CREATE TABLE` / indexes.
    pub async fn execute_batch(&self, sql: &str) -> Result<(), Error> {
        let sql = sql.to_owned();
        self.with_conn(move |conn| {
            conn.execute_batch(&sql)?;
            Ok(())
        })
        .await
    }

    /// Run a single statement with `params` (INSERT/UPDATE/DELETE); returns
    /// the number of affected rows.
    pub async fn execute(&self, sql: &str, params: &[Value]) -> Result<usize, Error> {
        let sql = sql.to_owned();
        let params = params.to_vec();
        self.with_conn(move |conn| {
            let refs: Vec<&dyn ToSql> = params.iter().map(|v| v as &dyn ToSql).collect();
            Ok(conn.execute(&sql, refs.as_slice())?)
        })
        .await
    }

    /// Run a SELECT and map each row. `params` are owned [`Value`]s so the
    /// query can run on a blocking thread.
    pub async fn query<T, M>(&self, sql: &str, params: &[Value], map: M) -> Result<Vec<T>, Error>
    where
        M: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let sql = sql.to_owned();
        let params = params.to_vec();
        self.with_conn(move |conn| {
            let mut stmt = conn.prepare(&sql)?;
            let refs: Vec<&dyn ToSql> = params.iter().map(|v| v as &dyn ToSql).collect();
            let rows = stmt.query_map(refs.as_slice(), map)?;
            Ok(rows.collect::<rusqlite::Result<Vec<T>>>()?)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::types::Value;

    use super::*;

    #[tokio::test]
    async fn execute_and_query_roundtrip() -> Result<(), Error> {
        let store = Storage::open(":memory:").await?;
        store
            .execute_batch("CREATE TABLE t (name TEXT, n INTEGER)")
            .await?;
        store
            .execute(
                "INSERT INTO t (name, n) VALUES (?1, ?2)",
                &[Value::Text("a".into()), Value::Integer(1)],
            )
            .await?;
        store
            .execute(
                "INSERT INTO t (name, n) VALUES (?1, ?2)",
                &[Value::Text("b".into()), Value::Integer(2)],
            )
            .await?;

        let rows: Vec<(String, i64)> = store
            .query("SELECT name, n FROM t ORDER BY n", &[], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .await?;
        assert_eq!(rows, vec![("a".to_string(), 1), ("b".to_string(), 2)]);

        let filtered: Vec<String> = store
            .query(
                "SELECT name FROM t WHERE n = ?1",
                &[Value::Integer(2)],
                |row| row.get(0),
            )
            .await?;
        assert_eq!(filtered, vec!["b".to_string()]);
        Ok(())
    }

    #[tokio::test]
    async fn execute_reports_affected_rows() -> Result<(), Error> {
        let store = Storage::open(":memory:").await?;
        store.execute_batch("CREATE TABLE t (n INTEGER)").await?;
        let inserted = store
            .execute("INSERT INTO t (n) VALUES (?1)", &[Value::Integer(1)])
            .await?;
        assert_eq!(inserted, 1);
        let deleted = store
            .execute("DELETE FROM t WHERE n = ?1", &[Value::Integer(1)])
            .await?;
        assert_eq!(deleted, 1);
        Ok(())
    }
}
