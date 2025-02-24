use super::util::AsyncStream;

pub use russh_sftp::protocol::FileAttributes;

#[derive(Debug, Clone)]
pub struct Metadata {
    pub path: String,
    pub ts: i64,
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
pub trait FileImpl: AsyncStream {}

pub type BoxedFile = Box<dyn FileImpl + Unpin + Send>;
