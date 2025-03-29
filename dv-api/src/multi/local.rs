use crate::{Error, whatever};

use super::dev::{self, *};
use autox::AutoX;

use std::path::{Path, PathBuf};
use tracing::trace;

mod config;
pub use config::create;
mod file;

pub(crate) struct This {
    home: Option<PathBuf>,
    autox: AutoX,
}

impl This {
    pub async fn new(is_system: bool) -> Result<Self> {
        let autox = AutoX::new(is_system).await.map_err(Error::unknown)?;
        Ok(Self {
            home: home::home_dir(),
            autox,
        })
    }
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

#[async_trait]
impl UserImpl for This {
    async fn file_attributes(
        &self,
        path: &Utf8Path,
    ) -> (camino::Utf8PathBuf, Result<FileAttributes>) {
        let path2 = self.expand_home(path.as_str());
        (
            path2.to_string_lossy().to_string().into(),
            std::fs::metadata(&path2)
                .map(|meta| (&meta).into())
                .map_err(|e| e.into()),
        )
    }
    async fn glob_file_meta(&self, path2: &camino::Utf8Path) -> Result<Vec<Metadata>> {
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
                #[cfg(not(windows))]
                use std::os::unix::fs::MetadataExt;
                #[cfg(not(windows))]
                let modified = metadata.mtime();
                #[cfg(windows)]
                use std::os::windows::fs::MetadataExt;
                #[cfg(windows)]
                let modified = metadata.last_write_time() as i64;
                let Ok(rel_path) = file_path.strip_prefix(path2) else {
                    continue;
                };
                result.push(Metadata {
                    path: rel_path.to_string_lossy().to_string().into(),
                    ts: modified,
                });
            }
            Ok(result)
        } else {
            whatever!("{} not a directory", path2)
        }
    }
    async fn copy(&self, src_path: &str, _: &str, dst_path: &str) -> Result<()> {
        let src2 = self.expand_home(src_path);

        let dst2 = self.expand_home(dst_path);

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
    async fn auto(&self, name: &str, action: &str, args: Option<&str>) -> Result<()> {
        match (action, args) {
            ("setup", Some(args)) => self.autox.setup(name, args).await.map_err(Error::unknown)?,
            ("reload", None) => self.autox.reload(name).await.map_err(Error::unknown)?,
            ("destroy", None) => self.autox.destroy(name).await.map_err(Error::unknown)?,
            _ => unimplemented!(),
        };
        Ok(())
    }
    async fn exec(&self, script: Script<'_, '_>) -> Result<Output> {
        let mut builder = script.into_command()?;
        builder
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let output = builder.output()?;
        Ok(Output {
            code: exit_status2exit_code(output.status),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
    async fn pty(&self, command: Script<'_, '_>, win_size: WindowSize) -> Result<BoxedPty> {
        trace!("try to exec command");
        let pty = openpty_local(win_size, command)?;
        Ok(pty)
    }
    async fn open(&self, path: &str, opt: OpenFlags) -> Result<BoxedFile> {
        let path2 = Path::new(path);

        let file = loop {
            match tokio::fs::OpenOptions::from(opt).open(&path2).await {
                Ok(file) => break Ok(file),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    let parent = path2.parent().unwrap();
                    tokio::fs::create_dir_all(parent).await?;
                }
                Err(e) => break Err(e),
            }
        };
        let file = file?;
        Ok(Box::new(file))
    }
}

#[cfg(not(windows))]
pub fn exit_status2exit_code(es: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    es.code()
        .unwrap_or_else(|| es.signal().map_or(1, |v| 128 + v))
}

#[cfg(windows)]
pub fn exit_status2exit_code(es: std::process::ExitStatus) -> i32 {
    es.code().unwrap_or(1)
}
