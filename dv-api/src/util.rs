use tokio::io::{AsyncRead, AsyncWrite};

mod pm;
pub use pm::{Package, Pm};
mod command;
mod dev_info;
pub use dev_info::{LinuxOs, Os};
mod dev {
    pub use crate::process::PtyProcessConsumer;
    pub use crate::{Result, params::*, user::*};
    pub use async_trait::async_trait;
    pub use e4pty::prelude::*;
}

pub use command::BoxedCommandUtil;

pub trait AsyncStream: AsyncRead + AsyncWrite {}

impl<T: AsyncRead + AsyncWrite> AsyncStream for T {}
