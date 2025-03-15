use crate::whatever;

use super::dev::{self, *};
use russh::client;
use russh_sftp::{client::SftpSession, protocol::StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};
mod config;
pub use config::SSHConfig;
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
    #[cfg(feature = "path-home")]
    home: Option<camino::Utf8PathBuf>,
    command_util: BoxedCommandUtil<Self>,
}

impl SSHSession {
    #[cfg(feature = "path-home")]
    fn expand_home<'a, 'b: 'a>(&'b self, path: &'a str) -> std::borrow::Cow<'a, camino::Utf8Path> {
        if let Some(home) = &self.home {
            if let Some(path) = path.strip_prefix("~/") {
                return home.join(path).into();
            } else if path == "~" {
                return camino::Utf8Path::new(home).into();
            }
        }
        camino::Utf8Path::new(path).into()
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
    async fn file_attributes(&self, path: &str) -> (String, Result<FileAttributes>) {
        #[cfg(feature = "path-home")]
        let path = self.expand_home(path);
        #[cfg(feature = "path-home")]
        let path = path.as_str();
        (
            path.to_string(),
            self.sftp.metadata(path).await.map_err(|e| e.into()),
        )
    }
    async fn glob_file_meta(&self, path: &str) -> crate::Result<Vec<Metadata>> {
        let metadata = self.sftp.metadata(path).await?;
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
                                path: sub_path.strip_prefix(&prefix).unwrap().to_string(),
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
        #[cfg(feature = "path-home")]
        let path = self.expand_home(path);
        #[cfg(feature = "path-home")]
        let path = path.as_str();

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
