use std::path::Path;

use async_trait::async_trait;
use dev_vault::Cache;
use tokio::sync::Mutex;
use tracing::info;

pub struct SqliteCache {
    conn: Mutex<rusqlite::Connection>,
}

impl SqliteCache {
    pub fn new(db_path: &Path) -> Self {
        info!("use sqlite path {}", db_path.display());
        let conn = rusqlite::Connection::open(db_path).expect("open sqlite connection");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache (
                device TEXT NOT NULL,
                path TEXT NOT NULL,
                modified INTEGER NOT NULL,
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

#[async_trait]
impl Cache for SqliteCache {
    async fn check_update(
        &self,
        device: &str,
        path: &str,
        modified: u64,
    ) -> dev_vault::Result<bool> {
        let row = self.conn.lock().await.query_row(
            "SELECT modified FROM cache WHERE device = ? AND path = ?",
            [device, path],
            |row| row.get::<_, u64>(0),
        );
        match row {
            Ok(m) => Ok(m != modified),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(true),
            Err(e) => Err(dev_vault::Error::Whatever {
                message: e.to_string(),
            }),
        }
    }
    async fn set(&self, device: &str, path: &str, modified: u64) -> dev_vault::Result<()> {
        info!("cache set: {} {} {}", device, path, modified);
        self.conn
            .lock()
            .await
            .execute(
                "INSERT OR REPLACE INTO cache (device, path, modified) VALUES (?, ?, ?)",
                [device, path, &modified.to_string()],
            )
            .map(|_| ())
            .map_err(|e| dev_vault::Error::Whatever {
                message: e.to_string(),
            })
    }
}
