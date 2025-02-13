use std::{
    io::Write,
    mem::ManuallyDrop,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::process::ExitStatusExt,
    },
    process::Command,
};

use rustix::termios;
use snafu::{OptionExt, ResultExt};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
};
use tracing::debug;

use crate::error;

use super::{BoxedPtyProcess, PtyProcessImpl, Script};

pub struct PtyProcess {
    pub inner: std::process::Child,
    pub stdio: OwnedFd,
}

impl PtyProcess {
    pub async fn new(command: Script<'_, '_>) -> std::io::Result<Self> {
        let pair = rustix_openpty::openpty(None, None)?;

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let Ok(mut termios) = termios::tcgetattr(&pair.controller) {
            // Set character encoding to UTF-8.
            termios.input_modes.set(termios::InputModes::IUTF8, true);
            let _ = termios::tcsetattr(&pair.controller, termios::OptionalActions::Now, &termios);
        }
        let mut builder = match command {
            Script::Whole(cmd) => {
                let mut iter = cmd.split_whitespace();
                let mut cmd = Command::new(iter.next().unwrap());
                cmd.args(iter);
                cmd
            }
            Script::Split { program, args } => {
                let mut cmd = Command::new(program);
                cmd.args(args);
                cmd
            }
            Script::Script { program, input } => {
                let mut cmd = Command::new(program);
                let mut temp = tempfile::NamedTempFile::new()?;
                temp.write_all(
                    format!("trap '{{ rm -f -- {}; }}' EXIT;", temp.path().display()).as_bytes(),
                )?;
                for line in input {
                    temp.write_all(line.as_bytes())?;
                }
                cmd.arg(temp.into_temp_path().keep()?);
                cmd
            }
        };
        // Setup child stdin/stdout/stderr.
        builder.stdin(pair.user.try_clone()?);
        builder.stderr(pair.user.try_clone()?);
        builder.stdout(pair.user.try_clone()?);
        let stdio = pair.controller.try_clone()?;
        unsafe {
            use std::os::unix::process::CommandExt;
            builder.pre_exec(move || {
                // Create a new process group.
                use rustix::process;
                process::setsid()?;
                process::ioctl_tiocsctty(&pair.user)?;

                rustix::io::close(pair.user.as_raw_fd());
                rustix::io::close(pair.controller.as_raw_fd());
                // libc::signal(libc::SIGCHLD, libc::SIG_DFL);
                // libc::signal(libc::SIGHUP, libc::SIG_DFL);
                // libc::signal(libc::SIGINT, libc::SIG_DFL);
                // libc::signal(libc::SIGQUIT, libc::SIG_DFL);
                // libc::signal(libc::SIGTERM, libc::SIG_DFL);
                // libc::signal(libc::SIGALRM, libc::SIG_DFL);
                //
                Ok(())
            });
        }
        // TODO:set working directory
        // set signal handler

        let child = builder.spawn()?;

        Ok(Self {
            inner: child,
            stdio,
        })
    }
}

#[async_trait::async_trait]
impl PtyProcessImpl for PtyProcess {
    async fn window_change(
        &self,
        width: u32,
        height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> crate::Result<()> {
        termios::tcsetwinsize(
            &self.stdio,
            termios::Winsize {
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
    async fn wait(&mut self) -> crate::Result<i32> {
        self.inner
            .wait()
            .context(error::IoSnafu {
                about: "wait process",
            })?
            .code()
            .with_whatever_context(|| "unkown exit code")
    }
    async fn output(&mut self) -> crate::Result<Vec<u8>> {
        let mut stdio = File::from_std(self.stdio.try_clone().unwrap().into());
        let mut buf = Vec::with_capacity(1024);
        let res = stdio.read_to_end(&mut buf).await;
        if let Err(e) = res {
            //FIX:Why does an uncategorized error occur when the child process exits?
            if e.raw_os_error() != Some(5) {
                return Err(e).context(error::IoSnafu {
                    about: "process output read",
                });
            }
        }
        let _ = ManuallyDrop::new(stdio);
        Ok(buf)
    }
    async fn sync(
        &mut self,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        mut writer: Box<dyn AsyncWrite + Unpin + Send>,
    ) -> crate::Result<i32> {
        let mut stdin = unsafe { File::from_raw_fd(self.stdio.as_raw_fd()) };
        let mut stdout = unsafe { File::from_raw_fd(self.stdio.as_raw_fd()) };
        async fn copy<LR, LW, RR, RW>(
            lr: &mut LR,
            lw: &mut LW,
            rr: &mut RR,
            rw: &mut RW,
        ) -> std::io::Result<()>
        where
            LR: AsyncRead + Unpin,
            LW: AsyncWrite + Unpin,
            RR: AsyncRead + Unpin,
            RW: AsyncWrite + Unpin,
        {
            let mut reader_closed = false;
            let mut buf = [0; 1024];
            let mut buf2 = [0; 1024];
            loop {
                tokio::select! {
                    r = lr.read(&mut buf), if !reader_closed => {
                        match r? {
                            0 => {
                                 reader_closed = true;
                                 rw.shutdown().await?;
                             },
                             n => {
                                 debug!("stdin: {}", String::from_utf8_lossy(&buf[..n]));
                                 rw.write_all(&buf[..n]).await?;
                                 rw.flush().await?;
                            }
                        };
                    },
                    r = rr.read(&mut buf2)  => {
                        match r? {
                            0 => break,
                            n => {
                                 debug!("stdout: {}", String::from_utf8_lossy(&buf2[..n]));
                                 lw.write_all(&buf2[..n]).await?;
                                 lw.flush().await?;
                            }
                        };
                    }
                }
            }
            Ok(())
        }

        let err = copy(&mut reader, &mut writer, &mut stdout, &mut stdin).await;

        let ec = self.inner.wait().map(|es| {
            es.code()
                .unwrap_or_else(|| es.signal().map_or(1, |v| 128 + v))
        });
        let _ = ManuallyDrop::new(stdin);
        let _ = ManuallyDrop::new(stdout);
        ec.and_then(|ec| if ec == 0 { Ok(0) } else { err.map(|_| ec) })
            .context(error::IoSnafu {
                about: "process sync",
            })
    }
}
impl From<PtyProcess> for BoxedPtyProcess {
    fn from(value: PtyProcess) -> Self {
        Box::new(value)
    }
}
