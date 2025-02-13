use std::sync::Arc;

use super::dev::*;
use async_trait::async_trait;
use snafu::whatever;

#[derive(Debug, Clone)]
pub struct AppTaskConfig {
    pub pkgs: Vec<String>,
}

#[derive(Debug, Clone)]
struct AppTask {
    pub pkgs: Vec<String>,
}

impl AppTask {
    pub fn new(pkgs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            pkgs: pkgs.into_iter().map(|n| n.into()).collect(),
        }
    }
}

#[async_trait]
impl<I: ContextImpl> Task<I> for AppTask {
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> crate::Result<TaskStatus>
    where
        I: 'async_trait,
    {
        let uid = target.get_dst_uid()?;
        let user = context.get_user(uid, false)?;
        let mut rp = user.app(&self.pkgs).await?;
        let int = context.get_interactor();
        let ec = int.ask(&mut rp).await?;
        if ec != 0 {
            whatever!("install {:?} fail, ec: {}", self.pkgs, ec);
        }
        Ok(TaskStatus::Success)
    }
}

#[derive(Debug, Clone)]
struct DryRunAppTask {
    pub pkgs: Vec<String>,
}

impl DryRunAppTask {
    pub fn new(pkgs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            pkgs: pkgs.into_iter().map(|n| n.into()).collect(),
        }
    }
}

#[async_trait]
impl<I: ContextImpl> Task<I> for DryRunAppTask {
    async fn exec(&self, _target: &Target, context: Arc<Context<I>>) -> crate::Result<TaskStatus>
    where
        I: 'async_trait,
    {
        context
            .get_interactor()
            .log(&format!("[Task] dry run install {:?}", self.pkgs))
            .await;
        Ok(TaskStatus::Success)
    }
}

impl<I: ContextImpl> TaskCast<I> for AppTaskConfig {
    fn cast(self, dry_run: bool) -> BoxedTask<I> {
        if !dry_run {
            AppTask::new(self.pkgs).into()
        } else {
            DryRunAppTask::new(self.pkgs).into()
        }
    }
}

into_boxed_task!(AppTask, DryRunAppTask);
