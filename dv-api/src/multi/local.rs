use crate::{Error, whatever, wrap::UserCast};

use super::dev::*;
use autox::AutoX;
#[cfg(not(windows))]
use std::env;
use std::path::Path;
#[cfg(feature = "path-home")]
use std::path::PathBuf;
use tracing::{trace, warn};

mod file;

#[cfg_attr(feature = "rune", derive(rune::Any))]
#[derive(Debug)]
pub struct LocalConfig {
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub hid: String,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub mount: String,
}
fn detect() -> Params {
    let user = {
        #[cfg(target_os = "linux")]
        {
            env::var("USER").unwrap_or("unspecified".to_string())
        }
        #[cfg(target_os = "macos")]
        {
            "macos".to_string()
        }
        #[cfg(target_os = "windows")]
        {
            "windows".to_string()
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
            None::<String>
        }
    } {
        p.session(session);
    }
    p
}

#[cfg(windows)]
fn is_user_admin() -> bool {
    use windows_sys::Win32::Security::{
        AllocateAndInitializeSid, CheckTokenMembership, FreeSid, SECURITY_NT_AUTHORITY,
    };
    use windows_sys::Win32::System::SystemServices::{
        DOMAIN_ALIAS_RID_ADMINS, SECURITY_BUILTIN_DOMAIN_RID,
    };

    unsafe {
        let mut sid = std::ptr::null_mut();
        // 创建管理员组的 SID
        let success = AllocateAndInitializeSid(
            &SECURITY_NT_AUTHORITY,
            2,
            SECURITY_BUILTIN_DOMAIN_RID as u32,
            DOMAIN_ALIAS_RID_ADMINS as u32,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut sid,
        ) != 0;

        if !success {
            return false;
        }

        // 检查令牌成员资格
        let mut is_member = 0;
        let check_success = CheckTokenMembership(std::ptr::null_mut(), sid, &mut is_member) != 0;

        // 释放 SID 内存
        FreeSid(sid);

        check_success && is_member != 0
    }
}

#[async_trait]
impl UserCast for LocalConfig {
    async fn cast(self) -> crate::Result<User> {
        let is_system = {
            #[cfg(windows)]
            {
                is_user_admin()
            }
            #[cfg(not(windows))]
            {
                rustix::process::getuid().is_root()
            }
        };
        let mut p = detect();
        p.mount(self.mount);
        let dev = This::new(is_system).await?;
        User::new(self.hid, p, is_system, dev).await
    }
}

pub(crate) struct This {
    #[cfg(feature = "path-home")]
    home: Option<PathBuf>,
    autox: AutoX, // TODO: add more
}

impl This {
    pub async fn new(is_system: bool) -> Result<Self> {
        let autox = AutoX::new(is_system)
            .await
            .map_err(|e| Error::Unknown(e.to_string()))?;
        Ok(Self {
            #[cfg(feature = "path-home")]
            home: home::home_dir(),
            autox,
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

#[async_trait]
impl UserImpl for This {
    async fn file_attributes(&self, path: &str) -> (String, Result<FileAttributes>) {
        #[cfg(feature = "path-home")]
        let path2 = self.expand_home(path);
        #[cfg(not(feature = "path-home"))]
        let path2 = Path::new(path);
        (
            path2.to_string_lossy().to_string(),
            std::fs::metadata(&path2)
                .map(|meta| (&meta).into())
                .map_err(|e| e.into()),
        )
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
    async fn auto(&self, name: &str, action: &str, args: Option<&str>) -> Result<()> {
        match (action, args) {
            ("setup", Some(args)) => self
                .autox
                .setup(name, args)
                .await
                .map_err(|e| Error::Unknown(e.to_string()))?,
            ("enable", None) => self
                .autox
                .enable(name)
                .await
                .map_err(|e| Error::Unknown(e.to_string()))?,
            ("reload", None) => self
                .autox
                .reload(name)
                .await
                .map_err(|e| Error::Unknown(e.to_string()))?,
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
