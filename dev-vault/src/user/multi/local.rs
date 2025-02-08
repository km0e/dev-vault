use super::dev::*;
use std::path::{Path, PathBuf};

use crate::error;
use async_trait::async_trait;
use process::PtyProcess;
use rustix::path::Arg;
use snafu::ResultExt;
use systemd::Systemd;
use tracing::trace;

mod file;
mod process;
mod systemd;

#[derive(Debug)]
pub struct LocalConfig {
    pub uid: String,
    pub hid: String,
    pub mount: PathBuf,
}

#[async_trait]
impl UserCast for LocalConfig {
    async fn cast(self) -> crate::Result<User> {
        let is_system = rustix::process::getuid().is_root();
        let dev = This::new(is_system).await?;
        User::new(
            self.uid.clone(),
            self.hid,
            is_system,
            Some(self.mount.clone()),
            Environment::detect(),
            dev,
        )
        .await
    }
}

pub struct This {
    systemd: Systemd, // TODO: add more
}

impl This {
    pub async fn new(is_system: bool) -> crate::Result<Self> {
        let systemd = Systemd::new(is_system).await?;
        Ok(Self { systemd })
    }
}

#[async_trait]
impl UserImpl for This {
    async fn check(&self, path: &str) -> CheckResult {
        Path::new(path).try_into()
    }
    async fn check_src(&self, path: &str) -> CheckSrcResult {
        let metadata = PathBuf::from(path)
            .metadata()
            .with_context(|_| error::IoSnafu {
                about: format!("Cannot get metadata of {}", path),
            })?;
        if metadata.is_dir() {
            let mut result = Vec::new();
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let file_path = entry.path();
                let metadata = match file_path.metadata() {
                    Ok(meta) => meta,
                    Err(_) => continue,
                };
                if metadata.is_dir() {
                    continue;
                }
                let Ok(modified) = metadata.modified() else {
                    continue;
                };
                let Ok(modified) = modified.duration_since(std::time::UNIX_EPOCH) else {
                    continue;
                };
                let modified = modified.as_secs();
                let Ok(rel_path) = file_path.strip_prefix(path) else {
                    continue;
                };
                result.push(Metadata {
                    path: rel_path.to_string_lossy().to_string(),
                    ts: modified,
                });
            }
            Ok(CheckInfo::Dir(DirInfo {
                path: path.to_string_lossy().to_string(),
                files: result,
            }))
        } else {
            Ok(CheckInfo::File(Path::new(path).try_into()?))
        }
    }
    async fn copy(&self, src: &str, dst: &str) -> crate::Result<()> {
        loop {
            match std::fs::copy(src, dst) {
                Ok(_) => break Ok(()),
                Err(e)
                    if e.kind() == std::io::ErrorKind::NotFound && {
                        #[cfg(debug_assertions)]
                        {
                            Path::new(src).exists()
                        }
                        #[cfg(not(debug_assertions))]
                        {
                            true
                        }
                    } =>
                {
                    let parent = Path::new(dst).parent().unwrap();
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        break Err(e);
                    }
                }
                Err(e) => break Err(e),
            }
        }
        .with_context(|_| error::IoSnafu {
            about: format!("{} -> {}", src, dst),
        })
    }
    async fn auto(&self, name: &str, action: &str) -> crate::Result<()> {
        match action {
            "setup" => self.systemd.setup(name).await?,
            "reload" => self.systemd.reload(name).await?,
            _ => unimplemented!(),
        };
        Ok(())
    }
    async fn exec(&self, command: CommandStr<'_, '_>, shell: Option<&str>) -> ExecResult {
        trace!("try to exec command");
        let cmd = PtyProcess::new(command, shell)
            .await
            .with_context(|_| error::IoSnafu {
                about: "create new pty",
            })?;
        Ok(cmd.into())
    }
    async fn open(&self, path: &str, opt: OpenFlags) -> crate::Result<BoxedFile> {
        let file = tokio::fs::OpenOptions::from(opt)
            .open(path)
            .await
            .with_context(|_| error::IoSnafu { about: path })?;
        Ok(Box::new(file))
    }
}

into_boxed_device!(This);
