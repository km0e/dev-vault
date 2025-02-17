use std::{borrow::Cow, path::Path};

use async_trait::async_trait;
use path_dedot::ParseDot;
use snafu::{whatever, ResultExt};
use tracing::info;

use crate::{
    error,
    fs::{BoxedFile, CheckInfo, FileStat, Metadata, OpenFlags},
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
            "new user:{} os:{} session:{:?} home:{:?} mount:{:?}",
            params.user, params.os, params.session, params.home, params.mount
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
    fn normalize<'a>(&self, path: &'a Path) -> crate::Result<Cow<'a, Path>> {
        let path = path
            .parse_dot()
            .with_context(|_| error::IoSnafu { about: "dedot" })?;
        let path = if path.has_root() {
            path
        } else {
            let path = match (
                path.strip_prefix("~"),
                self.params.home.as_ref(),
                self.params.mount.as_ref(),
            ) {
                (Ok(path), Some(home), _) => home.join(path),
                (Ok(_), None, _) => whatever!("we need home"),
                (Err(_), _, Some(mount)) => mount.join(path),
                (Err(_), Some(home), None) => home.join(path),
                _ => whatever!("we need mount or home"),
            };
            Cow::Owned(path)
        };
        Ok(path)
    }
    pub async fn check_file(&self, path: &str) -> Result<FileStat> {
        self.inner
            .check(&self.normalize(Path::new(path))?.to_string_lossy())
            .await
    }
    pub async fn check_src(&self, path: &str) -> Result<CheckInfo> {
        self.inner
            .check_src(&self.normalize(Path::new(path))?.to_string_lossy())
            .await
    }
    pub async fn glob_with_meta(&self, path: &str) -> Result<Vec<Metadata>> {
        self.inner
            .glob_with_meta(&self.normalize(Path::new(path))?.to_string_lossy())
            .await
    }
    pub async fn copy(&self, src: &str, dst: &str) -> Result<()> {
        self.inner
            .copy(
                &self.normalize(Path::new(src))?.to_string_lossy(),
                &self.normalize(Path::new(dst))?.to_string_lossy(),
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
        self.inner
            .open(&self.normalize(Path::new(path))?.to_string_lossy(), opt)
            .await
    }
}
