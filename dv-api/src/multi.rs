mod local;
pub use local::*;
mod ssh;
pub use ssh::*;

use dev::into_boxed_user;
use dev::BoxedUser;
into_boxed_user!(This, SSHSession);

mod dev {
    pub use super::super::params::*;
    pub use super::super::user::*;
    pub use crate::{error, fs::*, process::*, util::BoxedCommandUtil, Result, User, UserCast};
    pub use async_trait::async_trait;
}
