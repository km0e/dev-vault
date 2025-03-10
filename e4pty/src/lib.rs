use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

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
    async fn window_change(
        &self,
        width: u32,
        height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<(), String>;
}

pub type BoxedPtyWriter = Box<dyn PtyWriter + Send + Sync + Unpin>;

#[async_trait]
pub trait PtyReader: AsyncRead {
    async fn wait(&mut self) -> Result<i32, String>;
    async fn output(&mut self) -> Result<(i32, String), String>;
}

pub type BoxedPtyReader = Box<dyn PtyReader + Send + Sync + Unpin>;

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

mod instance;
pub use instance::*;
