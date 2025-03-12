use std::time::Duration;

use crossterm::terminal;
use dv_api::process::{BoxedPty, Interactor};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
    time::timeout,
};
use tracing::debug;

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
    async fn ask(&self, pty: BoxedPty) -> dv_api::Result<i32> {
        let (width, height) = terminal::size()?;
        let (ctl, mut tx, mut rx) = pty.destruct();
        ctl.window_change(width as u32, height as u32).await?;
        let mut stdin = noblock_stdin();
        #[allow(unused_variables)]
        let g = self.excl.lock().await;
        terminal::enable_raw_mode()?;
        let h = tokio::spawn(async move {
            let mut buf2 = [0; 1024];
            let res: Result<(), std::io::Error> = loop {
                let n = stdin.read(&mut buf2).await?;
                if n == 0 {
                    break Ok(());
                }
                debug!("sync {} from stdin to tx", n);
                tx.write_all(&buf2[..n]).await?;
                tx.flush().await?;
            };
            res
        });

        let mut to = tokio::io::stdout();
        // let res = flog!(copy(&mut rx, &mut to)).await;
        let hsout = tokio::spawn(async move {
            let mut buf = [0; 1024];
            let res: Result<(), std::io::Error> = loop {
                let n = rx.read(&mut buf).await?;
                if n == 0 {
                    break Ok(());
                }
                to.write_all(&buf[..n]).await?;
                to.flush().await?;
            };
            res
        });
        let status = ctl.wait().await.unwrap();
        h.abort();
        debug!("wait for sync stdin to tx");
        let res = match timeout(Duration::from_secs(1), hsout).await {
            Ok(res) => match res {
                Ok(res) => res
                    .map(|_| 0)
                    .map_err(|e| dv_api::Error::Unknown(e.to_string())),
                Err(e) => {
                    debug!("sync stdin to tx fail {:?}", e);
                    Err(dv_api::Error::Unknown(e.to_string()))
                }
            },
            Err(e) => {
                debug!("sync stdin to tx timeout {:?}", e);
                Err(dv_api::Error::Unknown(e.to_string()))
            }
        };
        debug!("sync stdin to tx done");
        terminal::disable_raw_mode()?;
        if status == 0 {
            Ok(0)
        } else {
            let es = res
                .map(|_| status)
                .map_err(|e| dv_api::Error::Unknown(e.to_string()))?;
            Ok(es)
        }
    }
}
#[cfg(windows)]
fn noblock_stdin() -> impl AsyncRead {
    use windows::Win32::{
        Storage::FileSystem::ReadFile,
        System::Console::{GetStdHandle, STD_INPUT_HANDLE},
    };

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = [0; 1024];
        let hin = unsafe { GetStdHandle(STD_INPUT_HANDLE).unwrap() };
        loop {
            let mut bytes = 0;
            unsafe {
                ReadFile(hin, Some(&mut buf), Some(&mut bytes), None).unwrap();
            }
            if bytes == 0 {
                break;
            }
            debug!("read {} bytes from stdin", bytes);
            tx.send(buf[..bytes as usize].to_vec()).unwrap();
        }
    });
    struct AsyncStdin {
        rx: std::sync::mpsc::Receiver<Vec<u8>>,
        buffer: (Vec<u8>, usize),
    }
    impl AsyncRead for AsyncStdin {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            debug!("poll_read");
            if self.buffer.1 == self.buffer.0.len() {
                debug!("try to read from stdin");
                match self.rx.try_recv() {
                    Ok(data) => {
                        self.buffer.0 = data;
                        self.buffer.1 = 0;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        cx.waker().wake_by_ref();
                        return std::task::Poll::Pending;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        return std::task::Poll::Ready(Ok(()));
                    }
                }
            }
            let n = std::cmp::min(buf.remaining(), self.buffer.0.len() - self.buffer.1);
            buf.put_slice(&self.buffer.0[self.buffer.1..self.buffer.1 + n]);
            self.buffer.1 += n;
            debug!("sync {} bytes from stdin", n);
            std::task::Poll::Ready(Ok(()))
        }
    }
    AsyncStdin {
        rx,
        buffer: (vec![], 0),
    }
}

#[cfg(not(windows))]
fn noblock_stdin() -> impl AsyncRead {
    struct AsyncStdin;
    impl AsyncRead for AsyncStdin {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let stdin = std::io::stdin();
            let mut stdin = stdin.lock();
            match stdin.read(buf.initialize_unfilled()) {
                Ok(n) => {
                    buf.advance(n);
                    std::task::Poll::Ready(Ok(()))
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::task::Poll::Pending,
                Err(e) => std::task::Poll::Ready(Err(e)),
            }
        }
    }
    AsyncStdin {}
}
