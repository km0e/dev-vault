use std::path::Path;

use rusqlite::Result;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug)]
pub struct SqliteCache {
    conn: Mutex<rusqlite::Connection>,
}

impl SqliteCache {
    pub fn new(db_path: impl AsRef<Path>) -> Self {
        let db_path = db_path.as_ref();
        info!("use sqlite path {}", db_path.display());
        let conn = rusqlite::Connection::open(db_path).expect("open sqlite connection");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache (
                device TEXT NOT NULL,
                path TEXT NOT NULL,
                version INTEGER NOT NULL,
                PRIMARY KEY (device, path)
            )",
            [],
        )
        .expect("create initial table");
        Self {
            conn: Mutex::new(conn),
        }
    }
    #[cfg(test)]
    pub fn memory() -> Self {
        let conn = rusqlite::Connection::open_in_memory().expect("open sqlite connection");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache (
                device TEXT NOT NULL,
                path TEXT NOT NULL,
                version INTEGER NOT NULL,
                PRIMARY KEY (device, path)
            )",
            [],
        )
        .expect("create initial table");
        Self {
            conn: Mutex::new(conn),
        }
    }
}

impl SqliteCache {
    pub async fn get(&self, uid: &str, path: &str) -> Result<Option<i64>> {
        let row = self.conn.lock().await.query_row(
            "SELECT version FROM cache WHERE device = ? AND path = ?",
            [uid, path],
            |row| row.get::<_, i64>(0),
        );
        match row {
            Ok(fs) => Ok(Some(fs)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub async fn set(&self, uid: &str, path: &str, version: i64) -> Result<()> {
        info!("cache set: {} {} {}", uid, path, version);
        self.conn
            .lock()
            .await
            .execute(
                "INSERT OR REPLACE INTO cache (device, path, version) VALUES (?, ?, ?)",
                [uid, path, &version.to_string()],
            )
            .map(|_| ())
    }
}
