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
    pub use crate::util::BoxedCommandUtil;
    pub use crate::{error, fs::*, process::*, Result, User, UserCast};
}
