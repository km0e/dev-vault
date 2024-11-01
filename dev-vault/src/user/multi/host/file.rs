use snafu::ResultExt;
use tokio::fs::OpenOptions;

use crate::error;

use super::{FileImpl, OpenFlags};
impl From<OpenFlags> for OpenOptions {
    fn from(value: OpenFlags) -> Self {
        let mut open_options = OpenOptions::new();
        if value.contains(OpenFlags::READ) {
            open_options.read(true);
        }
        if value.contains(OpenFlags::WRITE) {
            open_options.write(true);
        }
        if value.contains(OpenFlags::APPEND) {
            open_options.append(true);
        }
        if value.contains(OpenFlags::CREATE) {
            if value.contains(OpenFlags::EXCLUDE) {
                open_options.create_new(true);
            } else {
                open_options.create(true);
            }
        }
        if value.contains(OpenFlags::TRUNCATE) {
            open_options.truncate(true);
        }
        open_options
    }
}

#[async_trait::async_trait]
impl FileImpl for tokio::fs::File {
    async fn ts(&mut self) -> crate::Result<u64> {
        let mtime = self
            .metadata()
            .await
            .and_then(|m| m.modified())
            .with_context(|_| error::IoSnafu {
                about: "get metadata",
            })?;
        let mtime = mtime.duration_since(std::time::UNIX_EPOCH)?.as_secs();
        Ok(mtime)
    }
}
