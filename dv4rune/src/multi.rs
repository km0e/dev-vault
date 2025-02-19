use crate::{cache::SqliteCache, interactor::TermInteractor};
use dv_api::{process::Interactor, User};
use std::{collections::HashMap, future::Future};

pub struct Context<'a> {
    pub dry_run: bool,
    pub cache: &'a SqliteCache,
    pub interactor: &'a TermInteractor,
    users: &'a HashMap<String, User>,
}

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
        self.users
            .get(uid.as_ref())
            .ok_or_else(|| rune::support::Error::msg(format!("user {} not found", uid.as_ref())))
    }
    pub async fn try_get_user(&self, uid: impl AsRef<str>) -> Option<&'s User> {
        self.assert_option(self.users.get(uid.as_ref()), || {
            format!("user {} not found", uid.as_ref())
        })
        .await
    }
    pub async fn assert_option<T, M: AsRef<str>>(
        &self,
        opt: Option<T>,
        msg: impl FnOnce() -> M,
    ) -> Option<T> {
        match opt {
            Some(v) => Some(v),
            None => {
                self.interactor.log(msg().as_ref()).await;
                None
            }
        }
    }
    pub async fn assert_bool<M: AsRef<str>>(
        &self,
        cond: bool,
        msg: impl FnOnce() -> M,
    ) -> Option<bool> {
        if cond {
            Some(true)
        } else {
            self.interactor.log(msg().as_ref()).await;
            None
        }
    }
    pub async fn assert_result_with_msg<T, E, M: AsRef<str>>(
        &self,
        res: std::result::Result<T, E>,
        msg: impl FnOnce(E) -> M,
    ) -> Option<T> {
        match res {
            Ok(v) => Some(v),
            Err(e) => {
                self.interactor.log(msg(e).as_ref()).await;
                None
            }
        }
    }
    pub async fn assert_result<T, E: ToString>(&self, res: std::result::Result<T, E>) -> Option<T> {
        self.assert_result_with_msg(res, |e| e.to_string()).await
    }
    pub async fn async_assert_result<T, E: ToString>(
        &self,
        res: impl Future<Output = std::result::Result<T, E>>,
    ) -> Option<T> {
        self.assert_result(res.await).await
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
