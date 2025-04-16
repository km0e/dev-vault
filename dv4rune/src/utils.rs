mod oneshot;
pub use oneshot::Oneshot;

use dv_api::process::Interactor;

pub trait LogResult<I: Interactor> {
    async fn log(self, int: &I) -> Self;
}

impl<I: Sync + Interactor, T, E: ToString> LogResult<I> for Result<T, E> {
    async fn log(self, int: &I) -> Self {
        match &self {
            Ok(_) => {}
            Err(e) => {
                int.log(e.to_string()).await;
            }
        }
        self
    }
}

pub trait LogFutResult<I: Interactor> {
    type Result;
    async fn log(self, int: &I) -> Self::Result;
}

impl<I: Sync + Interactor, Fut: Future<Output = Result<T, E>>, T, E: ToString> LogFutResult<I>
    for Fut
{
    type Result = Result<T, E>;
    async fn log(self, int: &I) -> Self::Result {
        self.await.log(int).await
    }
}
