use std::{ffi::OsStr, fmt::Display, io::Write, process::Command};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::Result;

#[derive(Debug, Clone)]
pub struct WindowSize {
    pub rows: u16,
    pub cols: u16,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self { rows: 1, cols: 1 } // Can't be 0 in windows
    }
}

#[async_trait]
pub trait PtyWriter: AsyncWrite {
    async fn window_change(&self, width: u32, height: u32) -> Result<()>;
}

pub type BoxedPtyWriter = Box<dyn PtyWriter + Send + Sync + Unpin>;

#[async_trait]
pub trait PtyReader: AsyncRead {
    async fn wait(&mut self) -> Result<i32>;
}

pub type BoxedPtyReader = Box<dyn PtyReader + Send + Sync + Unpin>;

pub enum ScriptExecutor {
    Sh,
    Powershell,
}

impl Display for ScriptExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptExecutor::Sh => write!(f, "sh"),
            ScriptExecutor::Powershell => write!(f, "powershell"),
        }
    }
}

impl ScriptExecutor {
    pub fn prepare_clean(&self) -> Vec<u8> {
        match self {
            ScriptExecutor::Sh => b"\ntrap 'rm -f -- \"$0\"' EXIT;".to_vec(),
            ScriptExecutor::Powershell => b"\r\nRemove-Item $MyInvocation.MyCommand.Path".to_vec(),
        }
    }
}

impl AsRef<OsStr> for ScriptExecutor {
    fn as_ref(&self) -> &OsStr {
        match self {
            ScriptExecutor::Sh => OsStr::new("sh"),
            ScriptExecutor::Powershell => OsStr::new("powershell"),
        }
    }
}

pub enum Script<'a, 'b> {
    Whole(&'a str),
    Split {
        program: &'a str,
        args: Box<dyn 'b + Iterator<Item = &'a str> + Send>,
    },
    Script {
        executor: ScriptExecutor,
        input: Box<dyn 'b + Iterator<Item = &'a str> + Send>,
    },
}

impl<'a, 'b> Script<'a, 'b> {
    pub fn sh(input: Box<dyn 'b + Iterator<Item = &'a str> + Send>) -> Self {
        Script::Script {
            executor: ScriptExecutor::Sh,
            input,
        }
    }
    pub fn powershell(input: Box<dyn 'b + Iterator<Item = &'a str> + Send>) -> Self {
        Script::Script {
            executor: ScriptExecutor::Powershell,
            input,
        }
    }
    pub fn into_command(self) -> std::io::Result<Command> {
        let cmd = match self {
            Script::Whole(cmd) => {
                let mut iter = cmd.split_whitespace();
                let mut cmd = Command::new(iter.next().unwrap());
                cmd.args(iter);
                cmd
            }
            Script::Split { program, args } => {
                let mut cmd = Command::new(program);
                cmd.args(args);
                cmd
            }
            Script::Script { executor, input } => {
                let mut temp = tempfile::NamedTempFile::new()?;
                for line in input {
                    temp.write_all(line.as_bytes())?;
                }
                temp.write_all(executor.prepare_clean().as_slice())?;
                let path = temp.into_temp_path().keep()?;
                let mut cmd = Command::new(executor);
                cmd.arg(path);
                cmd
            }
        };
        Ok(cmd)
    }
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
