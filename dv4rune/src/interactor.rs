use std::io::Read;

use crossterm::terminal;
use dv_api::process::{BoxedPtyReader, BoxedPtyWriter, Interactor};

use resplus::flog;
use tokio::{
    io::{AsyncWriteExt, copy},
    sync::Mutex,
};
use tracing::{debug, trace};

#[derive(Debug)]
pub struct TermInteractor {
    excl: Mutex<()>,
}

#[cfg(not(windows))]
fn setup_stdin_nonblock() -> std::io::Result<()> {
    use rustix::fs;
    use std::os::fd::AsFd;
    let stdin = std::io::stdin();
    let fd = stdin.as_fd();
    fs::fcntl_setfl(fd, fs::fcntl_getfl(fd)? | fs::OFlags::NONBLOCK)?;
    Ok(())
}

#[cfg(windows)]
fn setup_stdin_nonblock() -> std::io::Result<()> {
    Ok(())
}

impl TermInteractor {
    pub fn new() -> std::io::Result<Self> {
        setup_stdin_nonblock()?;
        Ok(Self {
            excl: Mutex::new(()),
        })
    }
}

#[async_trait::async_trait]
impl Interactor for TermInteractor {
    async fn log(&self, msg: &str) {
        let _ = self.excl.lock().await;
        println!("{}", msg);
    }
    //TODO:more easily error handling
    async fn ask(&self, pty: (BoxedPtyWriter, BoxedPtyReader)) -> dv_api::Result<i32> {
        let (width, height) = terminal::size()?;
        let (mut tx, mut rx) = pty;
        tx.window_change(width as u32, height as u32).await?;
        #[allow(unused_variables)]
        let g = self.excl.lock().await;
        terminal::enable_raw_mode()?;
        let h = tokio::spawn(async move {
            debug!("start sync stdin to tx");
            let mut buf2 = [0; 1024];
            let mut stdin = std::io::stdin();
            let r = loop {
                let n = stdin.read(&mut buf2);
                debug!("sync stdin to tx {:?}", n);
                match n {
                    Ok(0) => break Ok(()),
                    Ok(n) => {
                        trace!("sync {} from stdin to tx", n);
                        tx.write_all(&buf2[..n]).await?;
                        tx.flush().await?;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await
                    }
                    Err(e) => break Err(e),
                };
            };
            debug!("sync stdin to tx over");
            r
        });

        let mut to = tokio::io::stdout();
        let res = flog!(copy(&mut rx, &mut to)).await;
        let status = rx.wait().await.unwrap();
        h.abort();
        terminal::disable_raw_mode()?;
        if status == 0 {
            Ok(0)
        } else {
            let es = res.map(|_| status)?;
            Ok(es)
        }
    }
}
