use std::sync::Arc;

use crate::error;

use super::dev::{self, *};

use async_trait::async_trait;
use snafu::{whatever, ResultExt};
use tracing::debug;

mod check;
use check::{CopyItem, PathDetail};

#[derive(Debug, Clone)]
pub struct CopyTaskConfig {
    pub pair: Vec<(String, String)>,
}

struct CopyInner {
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

struct CopyTask {
    inner: CopyInner,
}

impl From<CopyInner> for CopyTask {
    fn from(value: CopyInner) -> Self {
        Self { inner: value }
    }
}

#[async_trait]
impl<I: ContextImpl> Task<I> for CopyTask {
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> crate::Result<TaskStatus>
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
            let m = {
                if src_user.hid != dst_user.hid {
                    let mut src = src_user.open(&src_path, OpenFlags::READ).await?;
                    let mut dst = dst_user
                        .open(&dst_path, OpenFlags::WRITE | OpenFlags::CREATE)
                        .await?;
                    tokio::io::copy(&mut src, &mut dst)
                        .await
                        .with_context(|_| error::IoSnafu { about: "copy file" })?;
                    dst.ts().await?
                } else {
                    let main = if src_user.is_system {
                        src_user
                    } else {
                        dst_user
                    };
                    main.copy(&src_path, &dst_path).await?;
                    match dst_user.check_file(&dst_path).await? {
                        FileStat::Meta(meta) => meta.ts,
                        FileStat::NotFound => {
                            whatever!("copy {} -> {} failed", &src_path, &dst_path)
                        }
                    }
                }
            };
            cache.set(dst_uid, &dst_path, version, m).await?;
        }
        Ok(TaskStatus::Success)
    }
}

struct DryRunCopyTask {
    inner: CopyInner,
}
impl From<CopyInner> for DryRunCopyTask {
    fn from(value: CopyInner) -> Self {
        Self { inner: value }
    }
}
#[async_trait]
impl<I: ContextImpl> Task<I> for DryRunCopyTask {
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> crate::Result<TaskStatus>
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

impl<I: ContextImpl> TaskCast<I> for CopyTaskConfig {
    fn cast(self, dry_run: bool) -> BoxedTask<I> {
        let inner = CopyInner::new(self.pair);
        if !dry_run {
            CopyTask { inner }.into()
        } else {
            DryRunCopyTask { inner }.into()
        }
    }
}

into_boxed_task!(CopyTask, DryRunCopyTask);
