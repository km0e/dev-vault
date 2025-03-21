use crate::user::Utf8Path;
use resplus::attach;
use std::borrow::Cow;

use async_trait::async_trait;
use e4pty::prelude::*;
use tracing::{debug, info};

use crate::{
    Package, Pm, Result,
    fs::{BoxedFile, CheckInfo, DirInfo, FileAttributes, Metadata, OpenFlags},
    params::Params,
    process::DynInteractor,
    user::{BoxedUser, Output},
    whatever,
};

#[async_trait]
pub trait UserCast {
    async fn cast(self) -> crate::Result<User>;
}

#[derive(Debug)]
pub struct User {
    pub params: Params,
    pub hid: String,
    pub is_system: bool,
    inner: BoxedUser,
    pub pm: Pm,
}

impl User {
    pub async fn new(
        hid: impl Into<String>,
        params: Params,
        is_system: bool,
        dev: impl Into<BoxedUser>,
    ) -> crate::Result<Self> {
        info!(
            "new user:{} os:{} session:{:?} mount:{:?}",
            params.user, params.os, params.session, params.mount
        );
        let inner = dev.into();
        let pm = super::util::Pm::new(&inner, &params.os).await?;
        Ok(Self {
            hid: hid.into(),
            params,
            is_system,
            inner,
            pm,
        })
    }
    fn normalize<'a>(&self, path: impl Into<&'a camino::Utf8Path>) -> Cow<'a, camino::Utf8Path> {
        let path: &'a camino::Utf8Path = path.into();
        let path = if path.has_root() {
            Cow::Borrowed(path)
        } else {
            let path = match (path.starts_with("~"), self.params.mount.as_ref()) {
                (false, Some(mount)) => {
                    camino::Utf8PathBuf::from(format!("{}/{}", mount.as_str(), path.as_str()))
                        .into()
                }
                _ => path.into(),
            };
            path
        };
        path
    }
    pub async fn check_file(
        &self,
        path: &Utf8Path,
    ) -> (camino::Utf8PathBuf, Result<FileAttributes>) {
        let path = self.normalize(path);
        debug!("check_file:{}", path);
        self.inner.file_attributes(&path).await
    }
    pub async fn get_mtime(&self, path: &Utf8Path) -> Result<Option<i64>> {
        let (path, fa) = self.check_file(path).await;
        match fa {
            Ok(fa) => {
                let ts = match fa.mtime {
                    Some(time) => time as i64,
                    None => whatever!("{path} mtime"),
                };
                Ok(Some(ts))
            }
            Err(e) if e.is_not_found() => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub async fn check_path<'a, 'b: 'a>(&'b self, path: &'a str) -> Result<CheckInfo> {
        let path = self.normalize(path);
        let (path, fa) = self.inner.file_attributes(&path).await;
        debug!("check_path:{}", path);
        let fa = fa?;
        let info = if fa.is_dir() {
            let files = self.inner.glob_file_meta(&path).await?;
            CheckInfo::Dir(DirInfo { path, files })
        } else {
            let ts = match fa.mtime {
                Some(time) => time as i64,
                None => whatever!("{path} mtime"),
            };
            CheckInfo::File(Metadata { path, ts })
        };
        Ok(info)
    }
    pub async fn check_dir(&self, path: &str) -> Result<DirInfo> {
        let path = self.normalize(path);
        let (path, fa) = self.inner.file_attributes(&path).await;
        let fa = fa?;
        if !fa.is_dir() {
            whatever!("{} not a directory", path);
        }
        let metadata = self.inner.glob_file_meta(&path).await?;
        Ok(DirInfo {
            path,
            files: metadata,
        })
    }
    pub async fn copy(&self, src_path: &Utf8Path, dst: &str, dst_path: &Utf8Path) -> Result<()> {
        let src_path = self.normalize(src_path);
        let dst_path = self.normalize(dst_path);
        attach!(
            self.inner.copy(src_path.as_str(), dst, dst_path.as_str(),),
            0,
            2
        )
        .await?;
        Ok(())
    }
    pub async fn auto(&self, name: &str, action: &str, args: Option<&str>) -> Result<()> {
        self.inner.auto(name, action, args).await
    }
    pub async fn app(&self, interactor: &DynInteractor, packages: &Package) -> crate::Result<bool> {
        self.pm.install(self, interactor, packages).await
    }
    pub async fn pty(&self, s: Script<'_, '_>, win_size: WindowSize) -> Result<BoxedPty> {
        self.inner.pty(s, win_size).await
    }
    pub async fn exec(&self, s: Script<'_, '_>) -> Result<Output> {
        self.inner.exec(s).await
    }
    pub async fn open(&self, path: &Utf8Path, opt: OpenFlags) -> crate::Result<BoxedFile> {
        let path = self.normalize(path);
        attach!(self.inner.open(path.as_str(), opt), 0).await
    }
}
