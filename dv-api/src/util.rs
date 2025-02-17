use tokio::io::{AsyncRead, AsyncWrite};

mod am;
pub use am::{new_am, BoxedAm};
mod command;
mod dev {
    pub use crate::{params::*, user::*, Result};
    pub use async_trait::async_trait;
}

pub use command::BoxedCommandUtil;

pub trait AsyncStream: AsyncRead + AsyncWrite {}

impl<T: AsyncRead + AsyncWrite> AsyncStream for T {}
