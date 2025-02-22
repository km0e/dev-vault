use std::fmt::Debug;

use crate::{
    Result,
    fs::{BoxedFile, FileAttributes, Metadata, OpenFlags},
    process::{BoxedPtyProcess, Script},
};

#[async_trait::async_trait]
pub trait UserImpl {
    //FIX:about path encoding, should I use Utf8Path?
    async fn file_attributes(&self, path: &str) -> Result<(String, FileAttributes)>;
    async fn glob_file_meta(&self, path: &str) -> Result<Vec<Metadata>>;
    async fn copy(&self, src: &str, dst: &str) -> Result<()>;
    async fn open(&self, path: &str, opt: OpenFlags) -> Result<BoxedFile>;
    async fn auto(&self, name: &str, action: &str) -> Result<()>;
    async fn exec(&self, command: Script<'_, '_>) -> Result<BoxedPtyProcess>;
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
