use std::os::{fd::AsRawFd, unix::process::ExitStatusExt};

use async_trait::async_trait;
use rustix_openpty::rustix::termios::{self, Winsize};
use tokio::{fs::File, io::AsyncRead};

use crate::{core::*, error::Result};

#[async_trait]
impl PtyWriter for File {
    async fn window_change(&self, width: u32, height: u32) -> Result<()> {
        termios::tcsetwinsize(
            self,
            termios::Winsize {
                ws_row: height as u16,
                ws_col: width as u16,
                ws_xpixel: 0, // TODO: ws_xpixel:
                ws_ypixel: 0, // TODO: ws_ypixel:
            },
        )?;
        Ok(())
    }
}

struct PtyReaderImpl {
    _child: std::process::Child,
    f: File,
}

impl AsyncRead for PtyReaderImpl {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.f).poll_read(cx, buf)
    }
}

#[async_trait]
impl PtyReader for PtyReaderImpl {
    async fn wait(&mut self) -> Result<i32> {
        let ec = self._child.wait().map(|es| {
            es.code()
                .unwrap_or_else(|| es.signal().map_or(1, |v| 128 + v))
        })?;
        Ok(ec)
    }
}

pub fn openpty<'w, 'r>(
    window_size: WindowSize,
    script: Script<'_, '_>,
) -> std::io::Result<(
    impl PtyWriter + Send + Sync + Unpin + 'w,
    impl PtyReader + Send + Sync + Unpin + 'r,
)> {
    let pair = rustix_openpty::openpty(
        None,
        Some(&Winsize {
            ws_row: window_size.rows,
            ws_col: window_size.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }),
    )?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if let Ok(mut termios) = termios::tcgetattr(&pair.controller) {
        // Set character encoding to UTF-8.
        termios.input_modes.set(termios::InputModes::IUTF8, true);
        let _ = termios::tcsetattr(&pair.controller, termios::OptionalActions::Now, &termios);
    }
    let mut builder = script.into_command()?;
    // Setup child stdin/stdout/stderr.
    builder.stdin(pair.user.try_clone()?);
    builder.stderr(pair.user.try_clone()?);
    builder.stdout(pair.user.try_clone()?);
    let stdio = pair.controller.try_clone()?;
    unsafe {
        use std::os::unix::process::CommandExt;
        builder.pre_exec(move || {
            // Create a new process group.
            use rustix_openpty::rustix::{io, process};
            process::setsid()?;
            process::ioctl_tiocsctty(&pair.user)?;

            io::close(pair.user.as_raw_fd());
            io::close(pair.controller.as_raw_fd());
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
    use rustix_openpty::rustix::io;
    let pw = io::dup(&stdio)?;
    io::fcntl_setfd(&pw, io::fcntl_getfd(&pw)? | io::FdFlags::CLOEXEC)?;
    let pr = std::fs::File::from(stdio);
    Ok((
        File::from(std::fs::File::from(pw)),
        PtyReaderImpl {
            _child: child,
            f: pr.into(),
        },
    ))
}
