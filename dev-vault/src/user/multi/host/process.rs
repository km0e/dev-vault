use std::{
    os::fd::{AsRawFd, FromRawFd},
    pin::Pin,
};

use snafu::{OptionExt, ResultExt};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
};
use tracing::debug;

use crate::error;

use super::{BoxedPtyProcess, Command, PtyProcessImpl};

pub struct PtyProcess {
    pub inner: tokio::process::Child,
    pub stdin: File,
    pub stdout: File,
}

impl PtyProcess {
    pub async fn new(command: Command<'_, '_>, shell: Option<&str>) -> crate::Result<Self> {
        let pair = rustix_openpty::openpty(None, None).unwrap();
        let mut stdin = unsafe { File::from_raw_fd(pair.controller.as_raw_fd()) };
        let stdout = stdin.try_clone().await.unwrap();
        let mut cmd = if let Some(shell) = shell {
            let cmd = tokio::process::Command::new(shell);
            command.write_to(&mut stdin).await.context(error::IoSnafu {
                about: "write command to shell",
            })?;
            cmd
        } else {
            match command {
                Command::Whole(cmd) => {
                    let mut iter = cmd.split_whitespace();
                    let mut cmd = tokio::process::Command::new(iter.next().unwrap());
                    cmd.args(iter);
                    cmd
                }
                Command::Split { program, args } => {
                    let mut cmd = tokio::process::Command::new(program);
                    cmd.args(args);
                    cmd
                }
            }
        };
        cmd.stdin(pair.user.try_clone().unwrap())
            .stdout(pair.user.try_clone().unwrap())
            .stderr(pair.user.try_clone().unwrap());
        let child = cmd.spawn().context(error::IoSnafu { about: "exec" })?;
        Ok(Self {
            inner: child,
            stdin,
            stdout,
        })
    }
}

impl AsyncWrite for PtyProcess {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.stdin).poll_write(cx, buf)
    }
    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stdin).poll_flush(cx)
    }
    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stdin).poll_shutdown(cx)
    }
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.stdin).poll_write_vectored(cx, bufs)
    }
    fn is_write_vectored(&self) -> bool {
        self.stdin.is_write_vectored()
    }
}

#[async_trait::async_trait]
impl PtyProcessImpl for PtyProcess {
    async fn sync(
        &mut self,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        mut writer: Box<dyn AsyncWrite + Unpin + Send>,
    ) -> crate::Result<i32> {
        let mut reader_closed = false;
        let mut buf = vec![0; 1024];
        let mut buf2 = vec![0; 1024];
        let mut err = Ok(0);
        loop {
            tokio::select! {
                 r = reader.read(&mut buf), if !reader_closed => {
                     match r {
                         Ok(0) => {
                             reader_closed = true;
                             self.stdin.shutdown().await.context(error::IoSnafu{about:"shutdown"})?;
                         },
                         // Send it to the server
                         Ok(n) => {
                             debug!("stdin: {}", String::from_utf8_lossy(&buf[..n]));
                             self.stdin.write_all(&buf[..n]).await.context(error::IoSnafu{about:"process write"})?;
                             self.stdin.flush().await.context(error::IoSnafu{about:"process write"})?;
                         }
                         Err(e) => Err(e).context(error::IoSnafu{about:"stdin read"})?,
                     };
                 },
                 r = self.stdout.read(&mut buf2)  => {
                     match r {
                         Ok(0) => {
                             break;
                         },
                         Ok(n) => {
                             debug!("stdout: {}", String::from_utf8_lossy(&buf2[..n]));
                             writer.write_all(&buf2[..n]).await.context(error::IoSnafu{about:"stdout write"})?;
                             writer.flush().await.context(error::IoSnafu{about:"stdout write"})?;
                         }
                         Err(e) =>{
                            err= Err(e).context(error::IoSnafu{about:"rp read"});
                            break;
                        }
                     };
                 }
            }
        }

        let ec = self
            .inner
            .wait()
            .await
            .context(error::IoSnafu {
                about: "stdout write",
            })?
            .code();
        ec.with_whatever_context(|| "unkown exit code")
            .and_then(|ec| if ec == 0 { Ok(ec) } else { err })
    }
    async fn wait(&mut self) -> crate::Result<i32> {
        self.inner
            .wait()
            .await
            .context(error::IoSnafu {
                about: "wait process",
            })?
            .code()
            .with_whatever_context(|| "unkown exit code")
    }
    async fn window_change(
        &self,
        width: u32,
        height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> crate::Result<()> {
        rustix_openpty::rustix::termios::tcsetwinsize(
            &self.stdout,
            rustix_openpty::rustix::termios::Winsize {
                ws_row: height as u16,
                ws_col: width as u16,
                ws_xpixel: pix_width as u16,
                ws_ypixel: pix_height as u16,
            },
        )
        .map_err(|e| error::Error::Whatever {
            message: format!("tcsetwinsize: {}", e),
        })
    }
}
impl From<PtyProcess> for BoxedPtyProcess {
    fn from(value: PtyProcess) -> Self {
        Box::new(value)
    }
}
