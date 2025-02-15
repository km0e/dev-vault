use std::path::Path;

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
}

impl SqliteCache {
    pub async fn get(&self, uid: &str, path: &str) -> dev_vault::Result<Option<u64>> {
        let row = self.conn.lock().await.query_row(
            "SELECT version FROM cache WHERE device = ? AND path = ?",
            [uid, path],
            |row| row.get::<_, u64>(0),
        );
        match row {
            Ok(fs) => Ok(Some(fs)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(dev_vault::Error::Whatever {
                message: e.to_string(),
            }),
        }
    }
    pub async fn set(&self, uid: &str, path: &str, version: u64) -> dev_vault::Result<()> {
        info!("cache set: {} {} {}", uid, path, version);
        self.conn
            .lock()
            .await
            .execute(
                "INSERT OR REPLACE INTO cache (device, path, version) VALUES (?, ?, ?)",
                [uid, path, &version.to_string()],
            )
            .map(|_| ())
            .map_err(|e| dev_vault::Error::Whatever {
                message: e.to_string(),
            })
    }
}
