use dv_api::{
    User,
    process::{DynInteractor, Interactor},
};

use crate::{cache::SqliteCache, user::UserManager};

pub struct ExecContext<Int> {
    um: UserManager,
    cache: SqliteCache,
    interactor: Int,
}
impl<Int: Interactor> ExecContext<Int> {
    pub async fn new(um: UserManager, cache: SqliteCache, interactor: Int) -> Self {
        Self {
            um,
            cache,
            interactor,
        }
    }
}

impl<Int: Interactor + Sync + Send + 'static> super::ContextImpl for ExecContext<Int> {
    fn get_user(&self, id: &str, for_system: bool) -> Option<&User> {
        self.um.get_user(id, for_system)
    }
    fn get_cache(&self) -> &SqliteCache {
        &self.cache
    }
    fn get_interactor(&self) -> &DynInteractor {
        &self.interactor
    }
}
