use std::path::Path;

use snafu::ResultExt;
use tokio::io::AsyncWriteExt;

use crate::error;
mod util;

#[macro_use]
mod process;
pub use process::*;
mod file;
pub use file::*;

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

pub type CheckSrcResult = crate::Result<CheckInfo>;

pub enum FileStat {
    Meta(Metadata),
    NotFound,
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

pub type CheckResult = crate::Result<FileStat>;

pub enum CommandStr<'a, 'b> {
    Whole(&'a str),
    Split {
        program: &'a str,
        args: Box<dyn 'b + Iterator<Item = &'a str> + Send>,
    },
}

impl<'a, 'b> From<&'b [&'a str]> for CommandStr<'a, 'b> {
    fn from(args: &'b [&'a str]) -> Self {
        CommandStr::Split {
            program: args[0],
            args: Box::new(args.iter().skip(1).copied()),
        }
    }
}

impl<'a> From<&'a str> for CommandStr<'a, 'a> {
    fn from(program: &'a str) -> Self {
        CommandStr::Whole(program)
    }
}

impl From<CommandStr<'_, '_>> for String {
    fn from(value: CommandStr<'_, '_>) -> Self {
        match value {
            CommandStr::Whole(cmd) => cmd.to_string(),
            CommandStr::Split { program, args } => {
                let mut result = program.to_string();
                for arg in args {
                    result.push(' ');
                    result.push_str(arg);
                }
                result
            }
        }
    }
}

impl<'a, 'b> CommandStr<'a, 'b> {
    pub fn new<I>(program: &'a str, args: I) -> Self
    where
        I: IntoIterator<Item = &'a str> + 'b,
        <I as std::iter::IntoIterator>::IntoIter: Send,
    {
        Self::Split {
            program,
            args: Box::new(args.into_iter()),
        }
    }

    pub async fn write_to<W: AsyncWriteExt + Unpin>(self, mut writer: W) -> std::io::Result<()> {
        match self {
            Self::Whole(cmd) => writer.write_all(cmd.as_bytes()).await?,
            Self::Split { program, args } => {
                writer.write_all(program.as_bytes()).await?;
                for arg in args {
                    writer.write_all(b" ").await?;
                    writer.write_all(arg.as_bytes()).await?;
                }
            }
        }
        Ok(())
    }
}

impl<'a, 'b> From<CommandStr<'a, 'b>> for Vec<u8> {
    fn from(command: CommandStr<'a, 'b>) -> Self {
        match command {
            CommandStr::Whole(cmd) => cmd.as_bytes().to_vec(),
            CommandStr::Split { program, args } => {
                let mut result = program.as_bytes().to_vec();
                for arg in args {
                    result.push(b' ');
                    result.extend_from_slice(arg.as_bytes());
                }
                result
            }
        }
    }
}

pub type ExecResult = crate::Result<BoxedPtyProcess>;

#[async_trait::async_trait]
pub trait UserImpl {
    async fn check(&self, path: &str) -> CheckResult;
    async fn check_src(&self, path: &str) -> CheckSrcResult;
    async fn copy(&self, src: &str, dst: &str) -> crate::Result<()>;
    async fn auto(&self, name: &str, action: &str) -> crate::Result<()>;
    async fn exec(&self, command: CommandStr<'_, '_>, shell: Option<&str>) -> ExecResult;
    async fn open(&self, path: &str, opt: OpenFlags) -> crate::Result<BoxedFile>;
}

pub type BoxedUser = Box<dyn UserImpl + Send + Sync>;

macro_rules! into_boxed_device {
    ($t:ty) => {
        impl From<$t> for BoxedUser {
            fn from(value: $t) -> Self {
                Box::new(value)
            }
        }
    };
}
