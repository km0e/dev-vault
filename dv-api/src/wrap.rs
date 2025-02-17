use std::borrow::Cow;

use async_trait::async_trait;
use snafu::whatever;
use tracing::info;

use crate::{
    fs::{BoxedFile, CheckInfo, DirInfo, FileAttributes, Metadata, OpenFlags},
    params::Params,
    process::{BoxedPtyProcess, DynInteractor, Script},
    user::BoxedUser,
    util::BoxedAm,
    Result,
};

#[async_trait]
pub trait UserCast {
    async fn cast(self) -> crate::Result<User>;
}

#[derive(Debug)]

pub struct User {
    params: Params,
    pub hid: String,
    pub is_system: bool,
    pub inner: BoxedUser,
    am: BoxedAm,
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
        let am = super::util::new_am(&inner, &params.os).await?;
        Ok(Self {
            hid: hid.into(),
            params,
            is_system,
            inner,
            am,
        })
    }
    // fn normalize<'a>(&self, path: &'a Path) -> crate::Result<Cow<'a, Path>> {
    //     let path = path
    //         .parse_dot()
    //         .with_context(|_| error::IoSnafu { about: "dedot" })?;
    //     let path = if path.has_root() {
    //         path
    //     } else {
    //         let path = match (
    //             path.strip_prefix("~"),
    //             self.params.home.as_ref(),
    //             self.params.mount.as_ref(),
    //         ) {
    //             (Ok(path), Some(home), _) => home.join(path),
    //             (Ok(_), None, _) => whatever!("we need home"),
    //             (Err(_), _, Some(mount)) => mount.join(path),
    //             (Err(_), Some(home), None) => home.join(path),
    //             _ => whatever!("we need mount or home"),
    //         };
    //         Cow::Owned(path)
    //     };
    //     Ok(path)
    // }
    fn normalize2<'a>(
        &self,
        path: impl Into<&'a camino::Utf8Path>,
    ) -> crate::Result<Cow<'a, camino::Utf8Path>> {
        let path = path.into();
        // let path = path
        //     .parse_dot()
        //     .with_context(|_| error::IoSnafu { about: "dedot" })?;
        let path = if path.has_root() {
            path.into()
        } else {
            let path = match (path.starts_with("~"), self.params.mount.as_ref()) {
                (false, Some(mount)) => mount.join(path).into(),
                _ => Cow::Borrowed(path),
            };

            // let path = match (
            //     path.strip_prefix("~"),
            //     self.params.mount.as_ref(),
            // ) {
            //     (Ok(path), Some(home), _) => home.join(path),
            //     (Ok(_), None, _) => whatever!("we need home"),
            //     (Err(_), _, Some(mount)) => mount.join(path),
            //     (Err(_), Some(home), None) => home.join(path),
            //     _ => whatever!("we need mount or home"),
            // };
            path
        };
        Ok(path)
    }
    pub async fn check_file(&self, path: &str) -> Result<FileAttributes> {
        self.inner
            .file_attributes(self.normalize2(path)?.as_str())
            .await
    }
    pub async fn check_path<'a, 'b: 'a>(&'b self, path: &'a str) -> Result<CheckInfo> {
        let path = self.normalize2(path)?;
        let path = path.as_str();
        let fa = self.inner.file_attributes(path).await?;
        let info = if fa.is_dir() {
            let files = self.inner.glob_file_meta(path).await?;
            CheckInfo::Dir(DirInfo {
                path: path.to_string(),
                files,
            })
        } else {
            let ts = match fa.mtime {
                Some(time) => time as u64,
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
        let path = self.normalize2(path)?;
        let path = path.as_str();
        let metadata = self.inner.glob_file_meta(path).await?;
        Ok(DirInfo {
            path: path.to_string(),
            files: metadata,
        })
    }
    pub async fn copy(&self, src: &str, dst: &str) -> Result<()> {
        self.inner
            .copy(
                self.normalize2(src)?.as_str(),
                self.normalize2(dst)?.as_str(),
            )
            .await
    }
    pub async fn auto(&self, name: &str, action: &str) -> Result<()> {
        self.inner.auto(name, action).await
    }
    pub async fn app(&self, interactor: &DynInteractor, packages: &str) -> crate::Result<bool> {
        self.am.install(self, interactor, packages).await
    }
    pub async fn exec(&self, command: Script<'_, '_>) -> Result<BoxedPtyProcess> {
        self.inner.exec(command).await
    }
    pub async fn open(&self, path: &str, opt: OpenFlags) -> crate::Result<BoxedFile> {
        self.inner.open(self.normalize2(path)?.as_str(), opt).await
    }
}
