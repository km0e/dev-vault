use std::io::Write;

use async_trait::async_trait;
use dev_vault::user::BoxedPtyProcess;
use snafu::ResultExt;
use termion::{raw::IntoRawMode, terminal_size};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct TermInteractor {
    excl: Mutex<()>,
}

impl TermInteractor {
    pub fn new() -> Self {
        Self {
            excl: Mutex::new(()),
        }
    }
}

#[async_trait]
impl dev_vault::Interactor for TermInteractor {
    async fn log(&self, msg: &str) {
        let _ = self.excl.lock().await;
        println!("{}", msg);
    }
    async fn ask(&self, p: &mut BoxedPtyProcess) -> dev_vault::Result<i32> {
        let (width, height) = terminal_size().with_context(|_| dev_vault::error::IoSnafu {
            about: "terminal size",
        })?;
        p.window_change(width as u32, height as u32, 0, 0).await?;
        #[allow(unused_variables)]
        let g = self.excl.lock().await;
        let mut raw =
            std::io::stdout()
                .into_raw_mode()
                .with_context(|_| dev_vault::error::IoSnafu {
                    about: "into_raw_mode and into_alternate_screen",
                })?;
        raw.flush()
            .with_context(|_| dev_vault::error::IoSnafu { about: "flush raw" })?;
        let stdin = tokio_fd::AsyncFd::try_from(0)
            .with_context(|_| dev_vault::error::IoSnafu { about: "stdin" })?;
        let stdout = tokio_fd::AsyncFd::try_from(1)
            .with_context(|_| dev_vault::error::IoSnafu { about: "stdout" })?;
        let ec = p.sync(Box::new(stdin), Box::new(stdout)).await?;
        Ok(ec)
    }
}
