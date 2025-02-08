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
                version INTEGER NOT NULL,
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
    async fn get(&self, hid: &str, path: &str) -> dev_vault::Result<Option<(u64, u64)>> {
        let row = self.conn.lock().await.query_row(
            "SELECT version, modified FROM cache WHERE device = ? AND path = ?",
            [hid, path],
            |row| Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?)),
        );
        match row {
            Ok(fs) => Ok(Some(fs)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(dev_vault::Error::Whatever {
                message: e.to_string(),
            }),
        }
    }
    async fn set(
        &self,
        hid: &str,
        path: &str,
        version: u64,
        modified: u64,
    ) -> dev_vault::Result<()> {
        info!("cache set: {} {} {} {}", hid, path, version, modified);
        self.conn
            .lock()
            .await
            .execute(
                "INSERT OR REPLACE INTO cache (device, path, version, modified) VALUES (?, ?, ?, ?)",
                [hid, path, &version.to_string(), &modified.to_string()],
            )
            .map(|_| ())
            .map_err(|e| dev_vault::Error::Whatever {
                message: e.to_string(),
            })
    }
}
