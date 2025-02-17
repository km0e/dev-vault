use std::path::Path;

use snafu::ResultExt;

use crate::error;

use super::util::AsyncStream;

#[derive(Debug, Clone)]
pub struct Metadata {
    pub path: String,
    pub ts: u64,
}

impl TryFrom<&Path> for Metadata {
    type Error = crate::Error;
    fn try_from(path: &Path) -> crate::Result<Self> {
        let mtime = path
            .metadata()
            .and_then(|meta| meta.modified())
            .with_context(|_| error::IoSnafu {
                about: path.display().to_string(),
            })?;
        let mtime = mtime.duration_since(std::time::UNIX_EPOCH)?.as_secs();
        Ok(Self {
            path: path.to_string_lossy().to_string(),
            ts: mtime,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DirInfo {
    pub path: String,
    pub files: Vec<Metadata>,
}

#[derive(Debug, Clone)]
pub enum CheckInfo {
    Dir(DirInfo),
    File(Metadata),
}

pub enum FileStat {
    Meta(Metadata),
    NotFound,
}

impl From<FileStat> for Option<Metadata> {
    fn from(value: FileStat) -> Self {
        match value {
            FileStat::Meta(meta) => Some(meta),
            FileStat::NotFound => None,
        }
    }
}

impl TryFrom<&Path> for FileStat {
    type Error = crate::Error;
    fn try_from(path: &Path) -> crate::Result<Self> {
        let mtime = match path.metadata() {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::NotFound),
            res => res,
        }
        .and_then(|meta| meta.modified())
        .with_context(|_| error::IoSnafu {
            about: path.display().to_string(),
        })?;
        let mtime = mtime.duration_since(std::time::UNIX_EPOCH)?.as_secs();
        Ok(Self::Meta(Metadata {
            path: path.to_string_lossy().to_string(),
            ts: mtime,
        }))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OpenFlags(u32);

bitflags::bitflags! {
    impl OpenFlags: u32 {
        const READ = 0x00000001;
        const WRITE = 0x00000002;
        const APPEND = 0x00000004;
        const CREATE = 0x00000008;
        const TRUNCATE = 0x00000010;
        const EXCLUDE = 0x00000020;
    }
}

#[async_trait::async_trait]
pub trait FileImpl: AsyncStream {
    async fn ts(&mut self) -> crate::Result<u64>;
}

pub type BoxedFile = Box<dyn FileImpl + Unpin + Send>;
