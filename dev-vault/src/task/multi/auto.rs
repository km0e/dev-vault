use std::sync::Arc;

use super::dev::*;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct AutoTaskConfig {
    pub name: String,
    pub action: String,
}

#[derive(Debug, Clone)]
struct AutoTask {
    pub name: String,
    pub action: String,
}

impl AutoTask {
    pub fn new(name: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            action: action.into(),
        }
    }
}

#[async_trait]
impl<I: ContextImpl> Task<I> for AutoTask {
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> crate::Result<TaskStatus>
    where
        I: 'async_trait,
    {
        let uid = target.get_dst_uid()?;
        let user = context.get_user(uid, false)?;
        let res = user.auto(&self.name, &self.action).await;
        match res {
            Ok(_) => Ok(TaskStatus::Success),
            Err(e) => Err(e)?,
        }
    }
}

#[derive(Debug, Clone)]
struct DryRunAutoTask {
    pub name: String,
    pub action: String,
}

impl DryRunAutoTask {
    pub fn new(name: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            action: action.into(),
        }
    }
}

#[async_trait]
impl<I: ContextImpl> Task<I> for DryRunAutoTask {
    async fn exec(&self, _target: &Target, context: Arc<Context<I>>) -> crate::Result<TaskStatus>
    where
        I: 'async_trait,
    {
        context
            .get_interactor()
            .log(&format!(
                "[Task] dry run auto {} {}",
                &self.name, &self.action
            ))
            .await;
        Ok(TaskStatus::Success)
    }
}

impl<I: ContextImpl> TaskCast<I> for AutoTaskConfig {
    fn cast(self, dry_run: bool) -> BoxedTask<I> {
        if !dry_run {
            AutoTask::new(self.name, self.action).into()
        } else {
            DryRunAutoTask::new(self.name, self.action).into()
        }
    }
}

into_boxed_task!(AutoTask, DryRunAutoTask);
