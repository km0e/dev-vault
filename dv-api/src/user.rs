use resplus::attach;
use std::{borrow::Cow, collections::HashMap, sync::Arc};

use tracing::debug;

mod dev {
    pub use super::{super::core::*, User};
    pub use crate::{Result, fs::*, util::*};
    pub use async_trait::async_trait;
    pub use e4pty::prelude::*;
}

use dev::*;

mod config;
pub use config::Config;

use crate::{process::DynInteractor, whatever};
mod device;
pub use device::Dev;
mod multi;

#[derive(Debug)]
pub struct User {
    pub variables: HashMap<String, String>,
    pub is_system: bool,
    inner: BoxedUser,
    pub dev: Arc<Dev>,
}

impl User {
    pub async fn new(
        variables: HashMap<String, String>,
        is_system: bool,
        inner: BoxedUser,
        dev: Arc<Dev>,
    ) -> Result<Self> {
        Ok(Self {
            variables,
            is_system,
            inner,
            dev,
        })
    }
    fn normalize<'a>(&self, path: impl Into<&'a XPath>) -> Cow<'a, XPath> {
        let path: &'a XPath = path.into();
        let path = if path.has_root() {
            Cow::Borrowed(path)
        } else {
            if crate::core::VARIABLE_RE
                .captures(path.as_str())
                .is_some_and(|c| c.get(0).unwrap().start() == 0)
            {
                //TODO: replace variables
                return Cow::Borrowed(path);
            }
            let path = match (path.starts_with("~"), self.variables.get("MOUNT")) {
                (false, Some(mount)) => {
                    XPathBuf::from(format!("{}/{}", mount.as_str(), path.as_str())).into()
                }
                _ => path.into(),
            };
            path
        };
        path
    }
    pub async fn check_file(&self, path: &XPath) -> (XPathBuf, Result<FileAttributes>) {
        let path = self.normalize(path);
        debug!("check_file:{}", path);
        self.inner.file_attributes(&path).await
    }
    pub async fn get_mtime(&self, path: &XPath) -> Result<Option<i64>> {
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
    pub async fn copy(&self, src_path: &XPath, dst: &str, dst_path: &XPath) -> Result<()> {
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
    pub async fn app(&self, interactor: &DynInteractor, packages: Package<'_>) -> Result<bool> {
        packages.install(self, interactor, &self.dev.pm).await
    }
    pub async fn pty(&self, s: Script<'_, '_>, win_size: WindowSize) -> Result<BoxedPty> {
        self.inner.pty(s, win_size).await
    }
    pub async fn exec(&self, s: Script<'_, '_>) -> Result<Output> {
        self.inner.exec(s).await
    }
    pub async fn open(&self, path: &XPath, opt: OpenFlags) -> Result<BoxedFile> {
        let path = self.normalize(path);
        attach!(self.inner.open(path.as_str(), opt), 0).await
    }
}
