use std::io::Write;

use dv_api::process::{BoxedPtyProcess, Interactor};

use termion::{raw::IntoRawMode, terminal_size};
use tokio::sync::Mutex;

#[derive(Debug, Default)]
pub struct TermInteractor {
    excl: Mutex<()>,
}

#[async_trait::async_trait]
impl Interactor for TermInteractor {
    async fn log(&self, msg: &str) {
        let _ = self.excl.lock().await;
        println!("{}", msg);
    }
    //TODO:more easily error handling
    async fn ask(&self, p: &mut BoxedPtyProcess) -> dv_api::Result<i32> {
        let (width, height) = terminal_size()?;
        p.window_change(width as u32, height as u32, 0, 0).await?;
        #[allow(unused_variables)]
        let g = self.excl.lock().await;
        let mut raw = std::io::stdout().into_raw_mode()?;
        raw.flush()?;
        let stdin = tokio_fd::AsyncFd::try_from(0)?;
        let stdout = tokio_fd::AsyncFd::try_from(1)?;
        let ec = p.sync(Box::new(stdin), Box::new(stdout)).await?;
        Ok(ec)
    }
}
