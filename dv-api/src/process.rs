use async_trait::async_trait;
pub use e4pty::{BoxedPtyReader, BoxedPtyWriter, Script};
use tokio::io::AsyncReadExt;
use tracing::debug;

use crate::{Result, error::ErrorSource, whatever};

#[async_trait]
pub trait Interactor {
    async fn log(&self, msg: &str);
    async fn ask(&self, pty: (BoxedPtyWriter, BoxedPtyReader)) -> crate::Result<i32>;
}

pub type DynInteractor = dyn Interactor + Sync;

#[async_trait]
pub trait PtyProcessConsumer {
    async fn wait(self) -> Result<i32>;
    async fn output(self) -> Result<String>;
}

#[async_trait]
impl<T: Future<Output = Result<(BoxedPtyWriter, BoxedPtyReader)>> + Send> PtyProcessConsumer for T {
    async fn wait(self) -> Result<i32> {
        let es = self.await?.1.wait().await.map_err(ErrorSource::Unknown)?;
        Ok(es)
    }
    async fn output(self) -> Result<String> {
        let mut buf = String::new();
        let (_, mut rx) = self.await?;
        debug!("try to read output");
        let err = rx.read_to_string(&mut buf).await;
        let es = rx.wait().await.map_err(ErrorSource::Unknown)?;
        if es != 0 {
            err?;
            whatever!("exit status {}", es);
        }
        Ok(buf)
    }
}
