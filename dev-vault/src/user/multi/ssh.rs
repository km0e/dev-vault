use super::dev::{self, *};
use crate::error;
use async_trait::async_trait;
use russh::client;
use russh_sftp::{client::SftpSession, protocol::StatusCode};
use snafu::{whatever, ResultExt};
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};
mod config;
pub use config::SSHUserConfig;
mod file;
mod process;

struct Client {}

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct SSHSession {
    session: client::Handle<Client>,
    sftp: SftpSession,
    command_util: BoxedCommandUtil<Self>,
}

impl SSHSession {
    async fn metadata(
        &self,
        path: &str,
    ) -> Result<(String, russh_sftp::protocol::FileAttributes), russh_sftp::client::error::Error>
    {
        let path = self.sftp.canonicalize(path).await?;
        let metadata = self.sftp.metadata(&path).await?;
        Ok((path, metadata))
    }
}

#[async_trait]
impl UserImpl for SSHSession {
    async fn check(&self, path: &str) -> CheckResult {
        debug!("check {path}");
        // sftp canonicalize does not check if the file exists
        match self.metadata(path).await {
            Ok((path, metadata)) => {
                let Some(mtime) = metadata.mtime else {
                    whatever!("get mtime fail");
                };
                Ok(FileStat::Meta(Metadata {
                    path,
                    ts: mtime.into(),
                }))
            }
            Err(russh_sftp::client::error::Error::Status(s))
                if s.status_code == StatusCode::NoSuchFile =>
            {
                info!("{path} not found");
                Ok(FileStat::NotFound)
            }
            Err(e) => Err(e).context(error::SFTPSnafu { about: path })?,
        }
    }
    async fn check_src(&self, path: &str) -> CheckSrcResult {
        let (path, metadata) = self
            .metadata(path)
            .await
            .context(error::SFTPSnafu { about: path })?;
        if metadata.is_dir() {
            let mut stack = vec![path.to_string()];
            let prefix = format!("{path}/");
            let mut infos = Vec::new();
            while let Some(path) = stack.pop() {
                for entry in self
                    .sftp
                    .read_dir(&path)
                    .await
                    .context(error::SFTPSnafu { about: &path })?
                {
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
            return Ok(CheckInfo::Dir(DirInfo { path, files: infos }));
        }
        if metadata.is_regular() {
            if let Some(mtime) = metadata.mtime {
                return Ok(CheckInfo::File(Metadata {
                    path,
                    ts: mtime.into(),
                }));
            } else {
                warn!("can't get {path} mtime");
            }
        }
        whatever!("{path} is a {:?}", metadata.file_type());
    }
    async fn copy(&self, src: &str, dst: &str) -> crate::Result<()> {
        let code = self.command_util.copy(self, src, dst).await?.wait().await?;
        if code != 0 {
            whatever!("exec cp {} -> {} fail", src, dst);
        }
        Ok(())
    }
    async fn auto(&self, name: &str, action: &str) -> crate::Result<()> {
        let code = match action {
            "setup" => self.command_util.setup(self, name).await?,
            "reload" => self.command_util.reload(self, name).await?,
            _ => whatever!("unimplemented {}", action),
        }
        .wait()
        .await?;
        if code != 0 {
            whatever!("exec {} {} fail", action, name);
        }
        Ok(())
    }
    async fn exec(&self, command: Script<'_, '_>) -> ExecResult {
        let channel = self.session.channel_open_session().await?;
        channel
            .request_pty(
                true,
                std::env::var("TERM").as_deref().unwrap_or("xterm"),
                0,
                0,
                0,
                0,
                &[],
            )
            .await?;
        // info!("exec {}", command);
        if let Script::Script { program, input } = command {
            let mut retry = 5;
            let mut name = String::with_capacity(4 + 6);
            loop {
                //TODO:extract to a function?
                name.push_str(".tmp");
                for c in std::iter::repeat_with(fastrand::alphanumeric).take(6) {
                    name.push(c);
                }
                use russh_sftp::protocol::OpenFlags;
                match self
                    .sftp
                    .open_with_flags(
                        &name,
                        OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::EXCLUDE,
                    )
                    .await
                {
                    Ok(mut file) => {
                        file.write_all(format!("trap '{{ rm -f -- {}}}' EXIT;", name).as_bytes())
                            .await
                            .with_context(|_| error::IoSnafu {
                                about: "write trap",
                            })?;
                        for blk in input {
                            file.write_all(blk.as_bytes()).await.with_context(|_| {
                                error::IoSnafu {
                                    about: "write script",
                                }
                            })?;
                        }
                        break;
                    }
                    _ if retry > 0 => {
                        retry -= 1;
                        name.clear();
                    }
                    Err(e) => {
                        return Err(e).context(error::SFTPSnafu {
                            about: format!("create temp file {}", name),
                        })?;
                    }
                }
            }
            channel.exec(true, format!("{} {}", program, name)).await?;
        } else {
            channel.exec(true, command).await?;
        }
        Ok(channel.into())
    }
    async fn open(&self, path: &str, opt: OpenFlags) -> crate::Result<BoxedFile> {
        let open_flags = opt.into();
        let file = loop {
            match self.sftp.open_with_flags(path, open_flags).await {
                Ok(file) => break Ok(file),
                Err(russh_sftp::client::error::Error::Status(s))
                    if s.status_code == StatusCode::NoSuchFile =>
                {
                    let parent = path.rsplit_once("/").unwrap().0;
                    self.sftp
                        .create_dir(parent)
                        .await
                        .with_context(|_| error::SFTPSnafu {
                            about: format!("crate dir {}", parent),
                        })?;
                }
                Err(e) => break Err(e),
            }
        }
        .with_context(|_| error::SFTPSnafu {
            about: format!("fail to write {}", path),
        })?;
        Ok(Box::new(file))
    }
}

into_boxed_device!(SSHSession);
