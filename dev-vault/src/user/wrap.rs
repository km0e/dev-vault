use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use path_dedot::ParseDot;
use snafu::{whatever, ResultExt};
use tracing::info;

use crate::{env::Environment, error, Interactor, PrintState};

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

use super::{core::*, util::BoxedAm};
pub struct User {
    pub uid: String,
    pub hid: String,
    pub is_system: bool,
    pub mount: Option<PathBuf>,
    pub env: Environment,
    pub inner: BoxedUser,
    am: BoxedAm,
}

#[async_trait]
impl PrintState for User {
    async fn print(&self, interactor: &(dyn Interactor + Sync)) {
        interactor
            .log(&format!(
                "[User] id: {:<10}, hid: {:<10}, {}",
                self.uid, self.hid, self.env
            ))
            .await;
    }
}

impl User {
    pub async fn new(
        id: String,
        hid: String,
        is_system: bool,
        mount: Option<PathBuf>,
        env: Environment,
        dev: impl Into<BoxedUser>,
    ) -> crate::Result<Self> {
        let inner = dev.into();
        let am = super::util::new_am(&inner, &env).await?;
        Ok(Self {
            uid: id,
            hid,
            is_system,
            mount,
            env,
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
                self.env.home.as_ref(),
                self.mount.as_ref(),
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
        info!("[{}] check {}", self.uid, path);
        self.inner
            .check(&self.normalize(Path::new(path))?.to_string_lossy())
            .await
    }
    pub async fn check_src(&self, path: &str) -> CheckSrcResult {
        self.inner
            .check_src(&self.normalize(Path::new(path))?.to_string_lossy())
            .await
    }
    pub async fn copy(&self, src: &str, dst: &str) -> crate::Result<()> {
        info!("[{}] copy {} -> {}", self.uid, src, dst);
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
    pub async fn app(&self, package: &[String]) -> ExecResult {
        self.am.install(self, package).await
    }
    pub async fn exec(&self, command: CommandStr<'_, '_>, shell: Option<&str>) -> ExecResult {
        self.inner.exec(command, shell).await
    }
    pub async fn open(&self, path: &str, opt: OpenFlags) -> crate::Result<BoxedFile> {
        self.inner
            .open(&self.normalize(Path::new(path))?.to_string_lossy(), opt)
            .await
    }
}
