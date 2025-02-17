use crate::cache::SqliteCache;
use dv_api::{process::DynInteractor, User};

pub trait ContextImpl: Sync + Send {
    fn get_user(&self, uid: &str, for_system: bool) -> Option<&User>;
    fn get_cache(&self) -> &SqliteCache;
    fn get_interactor(&self) -> &DynInteractor;
}

mod exec;
pub use exec::ExecContext;

mod wrap;
pub use wrap::*;

#[cfg(test)]
pub mod tests {

    use crate::interactor::DebugInteractor;

    use super::*;

    #[derive(Debug, Default)]
    pub struct TestContext {
        interactor: DebugInteractor,
    }
    impl ContextImpl for TestContext {
        fn get_user(&self, _: &str, _: bool) -> Option<&User> {
            unimplemented!()
        }
        fn get_cache(&self) -> &SqliteCache {
            unimplemented!()
        }
        fn get_interactor(&self) -> &DynInteractor {
            &self.interactor
        }
    }
}
