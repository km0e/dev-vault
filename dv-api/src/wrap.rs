use resplus::attach;
use std::borrow::Cow;

use async_trait::async_trait;
use e4pty::prelude::*;
use tracing::info;

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
    fn normalize2<'a>(&self, path: impl Into<&'a camino::Utf8Path>) -> Cow<'a, camino::Utf8Path> {
        let path: &'a camino::Utf8Path = path.into();
        let path = if path.has_root() {
            path.into()
        } else {
            let path = match (path.starts_with("~"), self.params.mount.as_ref()) {
                (false, Some(mount)) => mount.join(path).into(),
                _ => Cow::Borrowed(path),
            };
            path
        };
        path
    }
    pub async fn check_file(&self, path: &str) -> (String, Result<FileAttributes>) {
        let path = self.normalize2(path);
        self.inner.file_attributes(path.as_str()).await
    }
    pub async fn check_path<'a, 'b: 'a>(&'b self, path: &'a str) -> Result<CheckInfo> {
        let path = self.normalize2(path);
        let (path, fa) = self.inner.file_attributes(path.as_str()).await;
        let fa = fa?;
        let info = if fa.is_dir() {
            let files = self.inner.glob_file_meta(&path).await?;
            CheckInfo::Dir(DirInfo {
                path: path.to_string(),
                files,
            })
        } else {
            let ts = match fa.mtime {
                Some(time) => time as i64,
                None => whatever!("{path} mtime"),
            };
            CheckInfo::File(Metadata {
                path: path.to_string(),
                ts,
            })
        };
        Ok(info)
    }
    pub async fn check_dir(&self, path: &str) -> Result<DirInfo> {
        let path = self.normalize2(path);
        let (path, fa) = self.inner.file_attributes(path.as_str()).await;
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
    pub async fn copy(&self, src_path: &str, dst: &str, dst_path: &str) -> Result<()> {
        let src_path = self.normalize2(src_path);
        let dst_path = self.normalize2(dst_path);
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
    pub async fn open(&self, path: &str, opt: OpenFlags) -> crate::Result<BoxedFile> {
        let path = self.normalize2(path);
        attach!(self.inner.open(path.as_str(), opt), 0).await
    }
}
