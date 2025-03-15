use std::fmt::Debug;

use crate::{
    Result,
    fs::{BoxedFile, FileAttributes, Metadata, OpenFlags},
};
use e4pty::prelude::*;

pub struct Output {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[async_trait::async_trait]
pub trait UserImpl {
    //FIX:about path encoding, should I use Utf8Path?
    //TODO:better path handling
    async fn file_attributes(&self, path: &str) -> (String, Result<FileAttributes>);
    async fn glob_file_meta(&self, path: &str) -> Result<Vec<Metadata>>;
    async fn copy(&self, src_path: &str, dst: &str, dst_path: &str) -> Result<()>;
    async fn open(&self, path: &str, opt: OpenFlags) -> Result<BoxedFile>;
    async fn auto(&self, name: &str, action: &str, args: Option<&str>) -> Result<()>;
    async fn exec(&self, command: Script<'_, '_>) -> Result<Output>;
    async fn pty(&self, command: Script<'_, '_>, win_size: WindowSize) -> Result<BoxedPty>;
}

pub type BoxedUser = Box<dyn UserImpl + Send + Sync>;

impl Debug for BoxedUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedUser").finish()
    }
}

macro_rules! into_boxed_user {
    ($t:ty) => {
        impl From<$t> for BoxedUser {
            fn from(value: $t) -> Self {
                Box::new(value)
            }
        }
    };
    ($t:ty, $($tail:tt)*) => {
        into_boxed_user!($t);
        into_boxed_user!($($tail)*);
    };
}

pub(crate) use into_boxed_user;
