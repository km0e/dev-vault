use crate::Result;
#[async_trait]
// TODO:maybe better to use a trait for the sync method
pub trait PtyProcessImpl {
    async fn window_change(
        &self,
        width: u32,
        height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<()>;
    async fn wait(&mut self) -> Result<i32>;
    async fn output(&mut self) -> Result<Vec<u8>>;
    async fn sync(
        &mut self,
        reader: Box<dyn tokio::io::AsyncRead + Unpin + Send>,
        writer: Box<dyn tokio::io::AsyncWrite + Unpin + Send>,
    ) -> Result<i32>;
}

pub type BoxedPtyProcess = Box<dyn PtyProcessImpl + Unpin + Send + Sync>;

#[async_trait]
pub trait PtyProcessConsumer {
    async fn wait(self) -> Result<i32>;
    async fn output(self) -> Result<Vec<u8>>;
}

#[async_trait]
impl<T: Future<Output = Result<BoxedPtyProcess>> + Send> PtyProcessConsumer for T {
    async fn wait(self) -> Result<i32> {
        self.await?.wait().await
    }
    async fn output(self) -> Result<Vec<u8>> {
        self.await?.output().await
    }
}

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
use async_trait::async_trait;

#[async_trait]
pub trait Interactor {
    async fn log(&self, msg: &str);
    async fn ask(&self, p: &mut BoxedPtyProcess) -> crate::Result<i32>;
}

pub type DynInteractor = dyn Interactor + Sync;
