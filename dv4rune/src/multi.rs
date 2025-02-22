use crate::{cache::SqliteCache, interactor::TermInteractor};
use dv_api::{User, process::Interactor};
use rune::support::Result as LRes;
use std::collections::HashMap;

pub struct Context<'a> {
    pub dry_run: bool,
    pub cache: &'a SqliteCache,
    pub interactor: &'a TermInteractor,
    users: &'a HashMap<String, User>,
}

macro_rules! action {
    ($ctx:ident, $suc:ident, $fmt:literal, $($arg:tt)*) => {
        $ctx.interactor.log(&format!(concat!("[{}] {} ",$fmt), if $ctx.dry_run { "n" } else { "a" }, if $suc { "exec" } else { "skip" }, $($arg)*)).await;
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
    pub async fn get_user(&self, uid: impl AsRef<str>) -> rune::support::Result<&'s User> {
        let uid = uid.as_ref();
        match self.users.get(uid) {
            Some(user) => Ok(user),
            None => {
                let m = format!("user {} not found", uid);
                self.interactor.log(&m).await;
                Err(rune::support::Error::msg(m))
            }
        }
    }
}

mod copy;
pub use copy::copy;
mod app;
pub use app::app;
mod auto;
pub use auto::auto;
mod exec;
pub use exec::exec;
