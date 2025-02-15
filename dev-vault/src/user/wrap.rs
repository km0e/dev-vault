use std::{borrow::Cow, path::Path};

use async_trait::async_trait;
use path_dedot::ParseDot;
use snafu::{whatever, ResultExt};
use tracing::info;

use crate::error;

#[derive(Debug, Default, Clone, Copy)]
pub struct Index {
    pub this: usize,
    pub system: Option<usize>,
}

pub type UserFilter = std::collections::HashSet<String>;

#[async_trait]
pub trait UserCast {
    async fn cast(self) -> crate::Result<User>;
}

use super::{core::*, params::Params, util::BoxedAm};
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
    pub async fn check_file(&self, path: &str) -> crate::Result<FileStat> {
        self.inner
            .check(&self.normalize(Path::new(path))?.to_string_lossy())
            .await
    }
    pub async fn check_src(&self, path: &str) -> CheckSrcResult {
        self.inner
            .check_src(&self.normalize(Path::new(path))?.to_string_lossy())
            .await
    }
    pub async fn glob_with_meta(&self, path: &str) -> crate::Result<Vec<Metadata>> {
        self.inner
            .glob_with_meta(&self.normalize(Path::new(path))?.to_string_lossy())
            .await
    }
    pub async fn copy(&self, src: &str, dst: &str) -> crate::Result<()> {
        self.inner
            .copy(
                &self.normalize(Path::new(src))?.to_string_lossy(),
                &self.normalize(Path::new(dst))?.to_string_lossy(),
            )
            .await
    }
    pub async fn auto(&self, name: &str, action: &str) -> crate::Result<()> {
        self.inner.auto(name, action).await
    }
    pub async fn app(&self, packages: &str) -> ExecResult {
        self.am.install(self, packages).await
    }
    pub async fn exec(&self, command: Script<'_, '_>) -> ExecResult {
        self.inner.exec(command).await
    }
    pub async fn open(&self, path: &str, opt: OpenFlags) -> crate::Result<BoxedFile> {
        self.inner
            .open(&self.normalize(Path::new(path))?.to_string_lossy(), opt)
            .await
    }
}
