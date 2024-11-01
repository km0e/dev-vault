use async_trait::async_trait;
use std::collections::HashMap;
use tracing::debug;
use wrap::Index;

#[macro_use]
mod core;
pub use core::{BoxedPtyProcess, CheckInfo, DirInfo, FileStat, Metadata, OpenFlags};
mod wrap;
pub use wrap::{User, UserCast, UserFilter};
mod util;
use crate::{Interactor, PrintState};
mod multi;
pub use multi::*;

#[derive(Default)]
pub struct UserManager {
    store: Vec<User>,
    user: HashMap<String, Index>,
}

#[async_trait]
impl PrintState for UserManager {
    async fn print(&self, interactor: &(dyn Interactor + Sync)) {
        for user in &self.store {
            user.print(interactor).await;
        }
    }
}

impl UserManager {
    pub fn get_user(&self, uid: &str, for_system: bool) -> Option<&User> {
        debug!("try get user {}", uid);
        let idx = self.user.get(uid)?;
        let dev = &self.store[if for_system { idx.system? } else { idx.this }];
        Some(dev)
    }
}

impl Extend<(Option<User>, Vec<User>)> for UserManager {
    fn extend<T: IntoIterator<Item = (Option<User>, Vec<User>)>>(&mut self, iter: T) {
        for (system, users) in iter.into_iter() {
            let mut idx = Index::default();
            if let Some(system) = system {
                idx.this = self.store.len();
                idx.system = Some(idx.this);
                self.user.insert(system.uid.clone(), idx);
                self.store.push(system);
            }
            for user in users {
                idx.this = self.store.len();
                self.user.insert(user.uid.clone(), idx);
                self.store.push(user);
            }
        }
    }
}
