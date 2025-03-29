mod local;
use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use dev::Result;
use dev::User;
mod ssh;
pub use ssh::*;

use dev::BoxedUser;
use dev::into_boxed_user;

use crate::dev::Dev;
into_boxed_user!(local::This, ssh::SSHSession);

mod dev {
    pub use super::super::user::*;
    pub use crate::{Result, User, fs::*, util::BoxedCommandUtil};
    pub use async_trait::async_trait;
    pub use e4pty::prelude::*;
}

#[cfg_attr(feature = "rune", derive(rune::Any))]
#[derive(Debug, Default)]
pub struct Config {
    #[cfg_attr(feature = "rune", rune(get, set))]
    is_system: Option<bool>,
    vars: HashMap<String, String>,
}

impl Deref for Config {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.vars
    }
}

impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vars
    }
}

impl Config {
    pub fn hid(&self) -> Option<&str> {
        self.get("HID").map(|s| s.as_str())
    }
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.vars.insert(key.into(), value.into())
    }
    pub async fn connect(mut self, dev: Option<Arc<Dev>>) -> Result<User> {
        if let Some(host) = self.remove("HOST") {
            ssh::create(host, self, dev).await
        } else {
            local::create(self, dev).await
        }
    }
}
