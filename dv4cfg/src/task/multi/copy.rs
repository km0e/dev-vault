use crate::error;

use super::dev::{self, *};
mod check;
use check::{CopyItem, PathDetail};
use dv_api::fs::OpenFlags;
use snafu::ResultExt;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyTaskConfig {
    pub attr: TaskAttr,
    #[serde(default)]
    pub target: Target,
    pub pair: Vec<(String, String)>,
}

impl<I: ContextImpl> TaskComplete<I> for CopyTaskConfig {
    fn cast(self, dry_run: bool, target: &Target) -> TaskParts<I> {
        let src_uid = self.target.src_uid.or_else(|| target.src_uid.clone());
        let dst_uid = self.target.dst_uid.or_else(|| target.dst_uid.clone());
        let inner = CopyInner::new(self.pair);

        TaskParts {
            id: self.attr.id,
            target: Target { src_uid, dst_uid },
            next: self.attr.next,
            task: if !dry_run {
                CopyTask::from(inner).into()
            } else {
                DryRunCopyTask::from(inner).into()
            },
        }
    }
}

impl CopyTaskConfig {
    pub fn new(
        attr: impl Into<TaskAttr>,
        pair: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            attr: attr.into(),
            target: Target::default(),
            pair: pair
                .into_iter()
                .map(|(src, dst)| (src.into(), dst.into()))
                .collect(),
        }
    }
    pub fn with_src(mut self, uid: impl Into<String>) -> Self {
        self.target.src_uid = Some(uid.into());
        self
    }
    pub fn with_dst(mut self, uid: impl Into<String>) -> Self {
        self.target.dst_uid = Some(uid.into());
        self
    }
}
pub struct CopyInner {
    pair: Vec<(String, String)>,
}

impl CopyInner {
    pub fn new(paths: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        Self {
            pair: paths
                .into_iter()
                .map(|(src, dst)| (src.into(), dst.into()))
                .collect(),
        }
    }
}

pub struct CopyTask {
    inner: CopyInner,
}

impl From<CopyInner> for CopyTask {
    fn from(value: CopyInner) -> Self {
        Self { inner: value }
    }
}

#[async_trait]
impl<I: ContextImpl> Task<I> for CopyTask {
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> Result<TaskStatus>
    where
        I: 'async_trait,
    {
        let (src_uid, dst_uid, copy_info) = check::check(target, &self.inner, &*context).await?;
        if copy_info.is_empty() {
            debug!("do not need copy");
            return Ok(TaskStatus::DoNothing);
        }
        let cache = context.get_cache();
        let interactor = context.get_interactor();
        for CopyItem {
            src:
                PathDetail {
                    user: src_user,
                    path: src_path,
                },
            dst:
                PathDetail {
                    user: dst_user,
                    path: dst_path,
                },
            version,
        } in copy_info
        {
            interactor
                .log(&format!(
                    "[Exec] copy {}:{} -> {}:{}",
                    &src_uid, &src_path, &dst_uid, &dst_path,
                ))
                .await;
            if src_user.hid != dst_user.hid {
                let mut src = src_user.open(&src_path, OpenFlags::READ).await?;
                let mut dst = dst_user
                    .open(&dst_path, OpenFlags::WRITE | OpenFlags::CREATE)
                    .await?;
                tokio::io::copy(&mut src, &mut dst)
                    .await
                    .with_context(|_| error::IoSnafu { about: "copy file" })?;
            } else {
                let main = if src_user.is_system {
                    src_user
                } else {
                    dst_user
                };
                main.copy(&src_path, &dst_path).await?;
            }
            cache.set(dst_uid, &dst_path, version).await?;
        }
        Ok(TaskStatus::Success)
    }
}

pub struct DryRunCopyTask {
    inner: CopyInner,
}
impl From<CopyInner> for DryRunCopyTask {
    fn from(value: CopyInner) -> Self {
        Self { inner: value }
    }
}
#[async_trait]
impl<I: ContextImpl> Task<I> for DryRunCopyTask {
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> Result<TaskStatus>
    where
        I: 'async_trait,
    {
        let (src_uid, dst_uid, copy_info) = check::check(target, &self.inner, &*context).await?;
        if copy_info.is_empty() {
            return Ok(TaskStatus::DoNothing);
        }
        let interactor = context.get_interactor();
        for CopyItem {
            src: PathDetail { path: src_path, .. },
            dst: PathDetail { path: dst_path, .. },
            ..
        } in copy_info
        {
            interactor
                .log(&format!(
                    "[Exec] copy {}:{} -> {}:{}",
                    &src_uid, &src_path, &dst_uid, &dst_path,
                ))
                .await;
        }
        Ok(TaskStatus::Success)
    }
}
