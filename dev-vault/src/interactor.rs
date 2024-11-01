use std::fmt::Debug;

use async_trait::async_trait;

use crate::user::BoxedPtyProcess;

#[async_trait]
pub trait Interactor {
    async fn log(&self, msg: &str);
    async fn ask(&self, p: &mut BoxedPtyProcess) -> crate::Result<i32>;
}

#[async_trait]
pub trait PrintState {
    async fn print(&self, interactor: &(dyn Interactor + Sync));
}

#[derive(Debug, Default)]
pub struct DebugInteractor;

#[async_trait]
impl Interactor for DebugInteractor {
    async fn ask(&self, _p: &mut BoxedPtyProcess) -> crate::Result<i32> {
        unimplemented!()
    }
    async fn log(&self, _msg: &str) {
        unimplemented!()
    }
}
