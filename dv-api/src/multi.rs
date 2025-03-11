mod local;
pub use local::*;
mod ssh;
pub use ssh::*;

use dev::BoxedUser;
use dev::into_boxed_user;
into_boxed_user!(This, SSHSession);

mod dev {
    pub use super::super::params::*;
    pub use super::super::user::*;
    pub use crate::{Result, User, UserCast, fs::*, util::BoxedCommandUtil};
    pub use async_trait::async_trait;
    pub use e4pty::prelude::*;
}
