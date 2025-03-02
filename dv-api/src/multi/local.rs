use crate::{Error, wrap::UserCast};

use super::dev::*;
use process::PtyProcess;
use snafu::whatever;
#[cfg(feature = "path-home")]
use std::path::PathBuf;

use std::{env, os::unix::fs::MetadataExt, path::Path};
use systemd::Systemd;
use tracing::{trace, warn};

mod file;
mod process;
mod systemd;

#[derive(Debug)]
pub struct LocalConfig {
    pub hid: String,
    pub mount: camino::Utf8PathBuf,
}
fn detect() -> Params {
    let user = {
        #[cfg(target_os = "linux")]
        {
            env::var("USER").unwrap_or("unspecified".to_string())
        }
        #[cfg(target_os = "macos")]
        {
            Some("macos".to_string())
        }
        #[cfg(target_os = "windows")]
        {
            Some("windows".to_string())
        }
    };
    let mut p = Params::new(user);
    p.os = if cfg!(target_os = "linux") {
        etc_os_release::OsRelease::open()
            .inspect_err(|e| warn!("can't open [/etc/os-release | /usr/lib/os-release]: {}", e))
            .map(|os_release| os_release.id().into())
            .unwrap_or("linux".into())
    } else if cfg!(target_os = "macos") {
        "macos".into()
    } else if cfg!(target_os = "windows") {
        "windows".into()
    } else {
        "unknown".into()
    };
    if let Some(session) = {
        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_SESSION_TYPE").ok()
        }
        #[cfg(target_os = "macos")]
        {
            None
        }
        #[cfg(target_os = "windows")]
        {
            None
        }
    } {
        p.session(session);
    }
    p
}
#[async_trait]
impl UserCast for LocalConfig {
    async fn cast(self) -> crate::Result<User> {
        let is_system = rustix::process::getuid().is_root();
        let mut p = detect();
        p.mount(self.mount);
        let dev = This::new(is_system).await?;
        User::new(self.hid, p, is_system, dev).await
    }
}

pub(crate) struct This {
    #[cfg(feature = "path-home")]
    home: Option<PathBuf>,
    systemd: Systemd, // TODO: add more
}

impl This {
    pub async fn new(is_system: bool) -> Result<Self> {
        let systemd = Systemd::new(is_system).await?;
        Ok(Self {
            #[cfg(feature = "path-home")]
            home: home::home_dir(),
            systemd,
        })
    }
    #[cfg(feature = "path-home")]
    fn expand_home<'a, 'b: 'a>(&'b self, path: &'a str) -> std::borrow::Cow<'a, Path> {
        if let Some(home) = &self.home {
            if let Some(path) = path.strip_prefix("~/") {
                return home.join(path).into();
            } else if path == "~" {
                return home.into();
            }
        }
        Path::new(path).into()
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => Error::NotFound,
            _ => Error::File {
                message: format!("source: {}", e),
            },
        }
    }
}

#[async_trait]
impl UserImpl for This {
    async fn file_attributes(&self, path: &str) -> Result<(String, FileAttributes)> {
        #[cfg(feature = "path-home")]
        let path2 = self.expand_home(path);
        #[cfg(not(feature = "path-home"))]
        let path2 = Path::new(path);

        Ok(std::fs::metadata(&path2)
            .map(|meta| (path2.to_string_lossy().to_string(), (&meta).into()))?)
    }
    async fn glob_file_meta(&self, path: &str) -> Result<Vec<Metadata>> {
        let path2 = Path::new(path);

        let metadata = path2.metadata()?;
        if metadata.is_dir() {
            let mut result = Vec::new();
            for entry in walkdir::WalkDir::new(path2)
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
                let modified = metadata.mtime();
                let Ok(rel_path) = file_path.strip_prefix(path2) else {
                    continue;
                };
                result.push(Metadata {
                    path: rel_path.to_string_lossy().to_string(),
                    ts: modified,
                });
            }
            Ok(result)
        } else {
            whatever!("{} not a directory", path)
        }
    }
    async fn copy(&self, src_path: &str, _: &str, dst_path: &str) -> Result<()> {
        #[cfg(feature = "path-home")]
        let src2 = self.expand_home(src_path);
        #[cfg(not(feature = "path-home"))]
        let src2 = Path::new(src_path);

        #[cfg(feature = "path-home")]
        let dst2 = self.expand_home(dst_path);
        #[cfg(not(feature = "path-home"))]
        let dst2 = Path::new(dst_path);

        let Err(e) = std::fs::copy(&src2, &dst2) else {
            return Ok(());
        };
        if e.kind() != std::io::ErrorKind::NotFound {
            Err(e)?;
        }
        let parent = dst2.parent().unwrap();
        std::fs::create_dir_all(parent)?;
        std::fs::copy(&src2, &dst2)?;
        Ok(())
    }
    async fn auto(&self, name: &str, action: &str) -> Result<()> {
        match action {
            "setup" => self.systemd.setup(name).await?,
            "reload" => self.systemd.reload(name).await?,
            _ => unimplemented!(),
        };
        Ok(())
    }
    async fn exec(&self, command: Script<'_, '_>) -> Result<BoxedPtyProcess> {
        trace!("try to exec command");
        let cmd = PtyProcess::new(command).await?;
        Ok(cmd.into())
    }
    async fn open(&self, path: &str, opt: OpenFlags) -> Result<BoxedFile> {
        let path2 = Path::new(path);

        let file = tokio::fs::OpenOptions::from(opt).open(&path2).await?;
        Ok(Box::new(file))
    }
}
