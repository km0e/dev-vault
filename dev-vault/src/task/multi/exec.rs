use std::sync::Arc;

use super::dev::*;
use async_trait::async_trait;
use snafu::whatever;

#[derive(Debug, Clone)]
pub struct ExecTaskConfig {
    pub shell: Option<String>,
    pub command: String,
}

#[derive(Debug, Clone)]
struct ExecTask {
    pub shell: Option<String>,
    pub command: String,
}

impl ExecTask {
    pub fn new(shell: Option<String>, command: impl Into<String>) -> Self {
        Self {
            shell,
            command: command.into(),
        }
    }
}

#[async_trait]
impl<I: ContextImpl> Task<I> for ExecTask {
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> crate::Result<TaskStatus>
    where
        I: 'async_trait,
    {
        let uid = target.get_dst_uid()?;
        let user = context.get_user(uid, false)?;
        let mut rp = user
            .exec(
                self.shell
                    .as_deref()
                    .map(|sh| Script::Script {
                        program: sh,
                        input: Box::new([self.command.as_str()].into_iter()),
                    })
                    .unwrap_or_else(|| Script::Whole(self.command.as_str())),
            )
            .await?;
        let interactor = context.get_interactor();
        let ec = interactor.ask(&mut rp).await?;
        if ec != 0 {
            whatever!("unexpected exit status {}", ec);
        }
        Ok(TaskStatus::Success)
    }
}

#[derive(Debug, Clone)]
struct DryRunExecTask {
    pub shell: Option<String>,
    pub command: String,
}

impl DryRunExecTask {
    pub fn new(shell: Option<String>, command: impl Into<String>) -> Self {
        Self {
            shell,
            command: command.into(),
        }
    }
}

#[async_trait]
impl<I: ContextImpl> Task<I> for DryRunExecTask {
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> crate::Result<TaskStatus>
    where
        I: 'async_trait,
    {
        let uid = target.get_dst_uid()?;
        context
            .get_interactor()
            .log(&format!(
                "[Task] dry run exec {} {} with {}",
                uid,
                &self.command,
                self.shell.as_deref().unwrap_or_default(),
            ))
            .await;
        Ok(TaskStatus::Success)
    }
}

impl<I: ContextImpl> TaskCast<I> for ExecTaskConfig {
    fn cast(self, dry_run: bool) -> BoxedTask<I> {
        if !dry_run {
            ExecTask::new(self.shell, self.command).into()
        } else {
            DryRunExecTask::new(self.shell, self.command).into()
        }
    }
}

into_boxed_task!(ExecTask, DryRunExecTask);
