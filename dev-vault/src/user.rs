use std::collections::HashMap;
use tracing::debug;
use wrap::Index;

#[macro_use]
mod core;
pub use core::{BoxedPtyProcess, CheckInfo, DirInfo, FileStat, Metadata, OpenFlags, Script};
mod wrap;
pub use wrap::{User, UserCast, UserFilter};
mod multi;
mod util;
pub use multi::*;
mod params;

#[derive(Default)]
pub struct UserManager {
    store: Vec<User>,
    user: HashMap<String, Index>,
}

impl UserManager {
    pub fn get_user(&self, uid: &str, for_system: bool) -> Option<&User> {
        debug!("try get user {}", uid);
        let idx = self.user.get(uid)?;
        let dev = &self.store[if for_system { idx.system? } else { idx.this }];
        Some(dev)
    }
}

impl Extend<(Option<(String, User)>, Vec<(String, User)>)> for UserManager {
    fn extend<T: IntoIterator<Item = (Option<(String, User)>, Vec<(String, User)>)>>(
        &mut self,
        iter: T,
    ) {
        for (system, users) in iter.into_iter() {
            let mut idx = Index::default();
            if let Some((uid, system)) = system {
                idx.this = self.store.len();
                idx.system = Some(idx.this);
                self.user.insert(uid, idx);
                self.store.push(system);
            }
            for (uid, user) in users {
                idx.this = self.store.len();
                self.user.insert(uid, idx);
                self.store.push(user);
            }
        }
    }
}
