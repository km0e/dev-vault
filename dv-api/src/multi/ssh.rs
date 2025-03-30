use std::{borrow::Cow, collections::HashMap};

use crate::whatever;

use super::dev::{self, *};
use russh::client;
use russh_sftp::{client::SftpSession, protocol::StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};
mod config;
pub use config::create;
mod file;

struct Client {}

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _: &russh::keys::ssh_key::PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        Ok(true)
    }
}

pub(crate) struct SSHSession {
    session: client::Handle<Client>,
    sftp: SftpSession,
    env: HashMap<String, String>,
    home: Option<String>,
    command_util: BoxedCommandUtil<Self>,
}

impl SSHSession {
    fn canonicalize<'a, 'b: 'a>(&'b self, path: &'a str) -> Result<std::borrow::Cow<'a, str>> {
        let path: Cow<str> = if let Some(path) = path.strip_prefix("~") {
            let Some(home) = self.home.as_deref() else {
                whatever!("unknown home")
            };
            if path.starts_with("/") {
                format!("{}{}", home, path).into()
            } else {
                home.into()
            }
        } else {
            path.into()
        };

        let mut new = String::with_capacity(path.len());
        let mut last_match = 0;
        for caps in VARIABLE_RE.captures_iter(&path) {
            let m = caps.get(0).unwrap();
            let var = caps.get(1).unwrap().as_str();
            let Some(value) = self.env.get(var) else {
                whatever!("unknown variable {}", var)
            };
            new.push_str(&path[last_match..m.start()]);
            new.push_str(value);
            last_match = m.end();
        }
        if last_match == 0 {
            return Ok(path);
        }
        new.push_str(&path[last_match..]);
        Ok(new.into())
    }
    async fn prepare_command(&self, command: Script<'_, '_>) -> Result<String> {
        let cmd = match command {
            Script::Whole(cmd) => cmd.to_string(),
            Script::Split { program, args } => {
                let mut cmd = program.to_string();
                for arg in args {
                    cmd.push(' ');
                    cmd.push_str(arg);
                }
                cmd
            }
            Script::Script { executor, input } => {
                let mut retry = 5;
                let mut name = String::with_capacity(4 + 6);
                loop {
                    //TODO:extract to a function?
                    name.push_str(".tmp");
                    for c in std::iter::repeat_with(fastrand::alphanumeric).take(6) {
                        name.push(c);
                    }
                    use russh_sftp::protocol::OpenFlags;
                    let res = self
                        .sftp
                        .open_with_flags(
                            &name,
                            OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::EXCLUDE,
                        )
                        .await;
                    if let Ok(mut file) = res {
                        file.write_all(&executor.prepare_clean()).await?;
                        for blk in input {
                            file.write_all(blk.as_bytes()).await?;
                        }
                        break;
                    } else if retry == 0 {
                        res?;
                    }
                    retry -= 1;
                    name.clear();
                }
                let cmd = format!("{} {}", executor, name);
                cmd
            }
        };
        Ok(cmd)
    }
}

#[async_trait]
impl UserImpl for SSHSession {
    async fn file_attributes(
        &self,
        path: &Utf8Path,
    ) -> (camino::Utf8PathBuf, Result<FileAttributes>) {
        let path2 = self.canonicalize(path.as_str());
        if path2.is_err() {
            return (path.into(), Err(path2.unwrap_err()));
        }
        let path = path2.unwrap();
        (
            path.to_string().into(),
            self.sftp.metadata(path).await.map_err(|e| e.into()),
        )
    }
    async fn glob_file_meta(&self, path: &camino::Utf8Path) -> crate::Result<Vec<Metadata>> {
        let metadata = self.sftp.metadata(path.to_string()).await?;
        if metadata.is_dir() {
            let mut stack = vec![path.to_string()];
            let prefix = format!("{path}/");
            let mut infos = Vec::new();
            while let Some(path) = stack.pop() {
                for entry in self.sftp.read_dir(&path).await? {
                    let sub_path = format!("{}/{}", path, entry.file_name());
                    if entry.file_type().is_dir() {
                        stack.push(sub_path);
                        continue;
                    }
                    if entry.file_type().is_file() {
                        if let Some(mtime) = entry.metadata().mtime {
                            infos.push(Metadata {
                                path: sub_path.strip_prefix(&prefix).unwrap().to_string().into(),
                                ts: mtime.into(),
                            });
                        } else {
                            warn!("can't get {sub_path} mtime");
                        }
                        continue;
                    }
                    warn!("find {:?} type file {sub_path}", entry.file_type());
                }
            }
            Ok(infos)
        } else {
            whatever!("{path} is a {:?}", metadata.file_type())
        }
    }
    async fn copy(&self, src_path: &str, dst: &str, dst_path: &str) -> crate::Result<()> {
        let ec = self
            .command_util
            .copy(self, src_path, dst, dst_path)
            .await?;
        if ec != 0 {
            whatever!("exec cp {} -> {} fail", src_path, dst_path);
        }
        Ok(())
    }
    async fn auto(&self, name: &str, action: &str, _: Option<&str>) -> crate::Result<()> {
        let ec = match action {
            "setup" => self.command_util.setup(self, name),
            "reload" => self.command_util.reload(self, name),
            _ => whatever!("unimplemented {}", action),
        }
        .await?;
        if ec != 0 {
            whatever!("exec {} {} fail", action, name);
        }
        Ok(())
    }
    async fn exec(&self, command: Script<'_, '_>) -> Result<Output> {
        let channel = self.session.channel_open_session().await?;
        let cmd = self.prepare_command(command).await?;
        info!("exec {}", cmd);
        channel.exec(true, cmd).await?;
        let mut pty = channel.into_pty();
        let mut stdout = Vec::new();
        pty.reader.read_to_end(&mut stdout).await?;
        let code = pty.ctl.wait().await?;
        debug!("exec done");
        Ok(Output {
            code,
            stdout,
            stderr: Vec::new(),
        })
    }
    async fn pty(&self, command: Script<'_, '_>, win_size: WindowSize) -> Result<BoxedPty> {
        let channel = self.session.channel_open_session().await?;
        channel
            .request_pty(
                true,
                std::env::var("TERM").as_deref().unwrap_or("xterm"),
                win_size.cols as u32,
                win_size.rows as u32,
                0,
                0,
                &[],
            )
            .await?;
        // info!("exec {}", command);
        let cmd = self.prepare_command(command).await?;
        channel.exec(true, cmd).await?;
        Ok(channel.into_pty())
    }
    async fn open(&self, path: &str, opt: OpenFlags) -> crate::Result<BoxedFile> {
        let path2 = self.canonicalize(path)?;
        let path = path2.as_ref();

        let open_flags = opt.into();
        let file = loop {
            match self.sftp.open_with_flags(path, open_flags).await {
                Ok(file) => break Ok(file),
                Err(russh_sftp::client::error::Error::Status(s))
                    if s.status_code == StatusCode::NoSuchFile =>
                {
                    let parent = path.rsplit_once("/").unwrap().0;
                    self.sftp.create_dir(parent).await?;
                }
                Err(e) => break Err(e),
            }
        }?;
        Ok(Box::new(file))
    }
}
