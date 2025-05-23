use crate::{cache::SqliteCache, interactor::TermInteractor};
use dv_api::user::User;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Context<'a> {
    pub dry_run: bool,
    pub cache: &'a SqliteCache,
    pub interactor: &'a TermInteractor,
    users: &'a HashMap<String, User>,
}

macro_rules! action {
    ($ctx:ident, $suc:expr, $fmt:expr, $($arg:tt)*) => {
        $ctx.interactor.log(format!(concat!("[{}] {} ",$fmt), if $ctx.dry_run { "n" } else { "a" }, if $suc { "exec" } else { "skip" }, $($arg)*)).await;
    };
}

pub(crate) use action;

impl<'s> Context<'s> {
    pub fn new<'a>(
        dry_run: bool,
        cache: &'a SqliteCache,
        interactor: &'a TermInteractor,
        users: &'a HashMap<String, User>,
    ) -> Context<'a> {
        Context {
            dry_run,
            cache,
            interactor,
            users,
        }
    }
    pub fn get_user(&self, uid: impl AsRef<str>) -> rune::support::Result<&'s User> {
        let uid = uid.as_ref();
        match self.users.get(uid) {
            Some(user) => Ok(user),
            None => {
                let m = format!("user {} not found", uid);
                Err(rune::support::Error::msg(m))
            }
        }
    }
}

mod copy;
mod user;
pub use copy::CopyContext;
mod pm;
pub use pm::{Packages, pm};
mod auto;
pub use auto::auto;
mod exec;
pub use exec::exec;
mod os;
mod util;

mod dev {
    pub use super::Context;
    pub use super::util::*;
    pub use crate::utils::LogFutResult;
    pub use dv_api::process::Interactor;
    pub use rune::support::Result as LRes;
}

pub fn register(m: &mut rune::module::Module) -> Result<(), rune::ContextError> {
    user::register(m)?;
    os::register(m)?;
    pm::register(m)?;
    Ok(())
}
