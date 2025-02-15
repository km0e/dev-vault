use std::{fmt::Debug, path::Path};

use snafu::ResultExt;

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

pub type CheckResult = crate::Result<FileStat>;

pub enum Script<'a, 'b> {
    Whole(&'a str),
    Split {
        program: &'a str,
        args: Box<dyn 'b + Iterator<Item = &'a str> + Send>,
    },
    Script {
        program: &'a str,
        input: Box<dyn 'b + Iterator<Item = &'a str> + Send>,
    },
}

impl<'a, 'b> From<&'b [&'a str]> for Script<'a, 'b> {
    fn from(args: &'b [&'a str]) -> Self {
        Script::Split {
            program: args[0],
            args: Box::new(args.iter().skip(1).copied()),
        }
    }
}

impl<'a> From<&'a str> for Script<'a, 'a> {
    fn from(program: &'a str) -> Self {
        Script::Whole(program)
    }
}

impl<'a, 'b> Script<'a, 'b> {
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
}
impl From<Script<'_, '_>> for Vec<u8> {
    fn from(value: Script<'_, '_>) -> Self {
        String::from(value).into_bytes()
    }
}

impl From<Script<'_, '_>> for String {
    fn from(value: Script<'_, '_>) -> Self {
        match value {
            Script::Whole(cmd) => cmd.to_string(),
            Script::Split { program, args } => {
                let mut result = program.to_string();
                for arg in args {
                    result.push(' ');
                    result.push_str(arg);
                }
                result
            }
            Script::Script { program, input } => {
                let mut result = program.to_string();
                for arg in input {
                    result.push(' ');
                    result.push_str(arg);
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
    async fn glob_with_meta(&self, path: &str) -> crate::Result<Vec<Metadata>>;
    async fn copy(&self, src: &str, dst: &str) -> crate::Result<()>;
    async fn auto(&self, name: &str, action: &str) -> crate::Result<()>;
    async fn exec(&self, command: Script<'_, '_>) -> ExecResult;
    async fn open(&self, path: &str, opt: OpenFlags) -> crate::Result<BoxedFile>;
}

pub type BoxedUser = Box<dyn UserImpl + Send + Sync>;

impl Debug for BoxedUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedUser").finish()
    }
}

macro_rules! into_boxed_device {
    ($t:ty) => {
        impl From<$t> for BoxedUser {
            fn from(value: $t) -> Self {
                Box::new(value)
            }
        }
    };
}
