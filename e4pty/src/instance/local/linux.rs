use std::{
    io::Write,
    os::{fd::AsRawFd, unix::process::ExitStatusExt},
    process::Command,
};

use async_trait::async_trait;
use rustix_openpty::rustix::termios;
use tokio::{fs::File, io::AsyncRead};

use crate::{PtyReader, PtyWriter, Script};

#[async_trait]
impl PtyWriter for File {
    async fn window_change(
        &self,
        width: u32,
        height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<(), String> {
        termios::tcsetwinsize(
            self,
            termios::Winsize {
                ws_row: height as u16,
                ws_col: width as u16,
                ws_xpixel: pix_width as u16,
                ws_ypixel: pix_height as u16,
            },
        )
        .map_err(|e| format!("tcsetwinsize: {}", e))
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
    async fn wait(&mut self) -> Result<i32, String> {
        self._child
            .wait()
            .map_err(|e| format!("wait: {}", e))
            .map(|es| {
                es.code()
                    .unwrap_or_else(|| es.signal().map_or(1, |v| 128 + v))
            })
    }
}

pub fn openpty<'w, 'r>(
    window_size: (u16, u16),
    command: Script<'_, '_>,
) -> std::io::Result<(
    impl PtyWriter + Send + Sync + Unpin + 'w,
    impl PtyReader + Send + Sync + Unpin + 'r,
)> {
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
            let path = temp.into_temp_path().keep()?;
            cmd.arg(path);
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
