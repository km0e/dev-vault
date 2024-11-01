use snafu::OptionExt;

use super::ContextImpl;

pub struct Context<I: ContextImpl> {
    impl_: I,
}

pub trait WrapContext<I: ContextImpl> {
    fn wrap(self) -> Context<I>;
}

impl<T: ContextImpl> WrapContext<T> for T {
    fn wrap(self) -> Context<T> {
        Context { impl_: self }
    }
}

impl<I: ContextImpl> Context<I> {
    pub fn get_user(&self, uid: &str, for_system: bool) -> crate::Result<&crate::user::User> {
        self.impl_
            .get_user(uid, for_system)
            .with_whatever_context(|| format!("No such device: {}", uid))
    }
    pub fn get_cache(&self) -> &(dyn crate::cache::Cache + Sync) {
        self.impl_.get_cache()
    }
    pub fn get_interactor(&self) -> &(dyn crate::interactor::Interactor + Sync) {
        self.impl_.get_interactor()
    }
}
