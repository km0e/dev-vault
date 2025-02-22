use tokio::io::{AsyncRead, AsyncWrite};

mod am;
pub use am::{BoxedAm, new_am};
mod command;
mod dev_info;
pub use dev_info::Os;
mod dev {
    pub use crate::{Result, params::*, user::*};
    pub use async_trait::async_trait;
}

pub use command::BoxedCommandUtil;

pub trait AsyncStream: AsyncRead + AsyncWrite {}

impl<T: AsyncRead + AsyncWrite> AsyncStream for T {}
