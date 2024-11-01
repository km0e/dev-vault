use crate::{cache::Cache, interactor::Interactor, user::UserManager};

pub struct ExecContext<C, Int> {
    um: UserManager,
    cache: C,
    interactor: Int,
}
impl<C: Cache, Int: Interactor> ExecContext<C, Int> {
    pub async fn new(um: UserManager, cache: C, interactor: Int) -> Self {
        Self {
            um,
            cache,
            interactor,
        }
    }
}

impl<C: Cache + Sync + Send, Int: Interactor + Sync + Send> super::ContextImpl
    for ExecContext<C, Int>
{
    fn get_user(&self, id: &str, for_system: bool) -> Option<&crate::user::User> {
        self.um.get_user(id, for_system)
    }
    fn get_cache(&self) -> &(dyn Cache + Sync) {
        &self.cache
    }
    fn get_interactor(&self) -> &(dyn Interactor + Sync) {
        &self.interactor
    }
}
