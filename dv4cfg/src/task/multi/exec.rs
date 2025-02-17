use dv_api::process::Script;
use tracing::debug;

use super::dev::*;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecTaskConfig {
    pub attr: TaskAttr,
    pub uid: Option<String>,
    pub shell: Option<String>,
    pub command: String,
}
impl<I: ContextImpl> TaskComplete<I> for ExecTaskConfig {
    fn cast(self, dry_run: bool, target: &Target) -> TaskParts<I> {
        let dst_uid = self.uid.or_else(|| target.dst_uid.clone());
        debug!("{} dst: {:?}", self.attr.id, dst_uid);
        TaskParts {
            id: self.attr.id,
            target: Target {
                dst_uid,
                ..Default::default()
            },
            next: self.attr.next,
            task: if !dry_run {
                ExecTask::new(self.shell, self.command).into()
            } else {
                DryRunExecTask::new(self.shell, self.command).into()
            },
        }
    }
}
impl ExecTaskConfig {
    pub fn new(attr: impl Into<TaskAttr>, command: impl Into<String>) -> Self {
        Self {
            attr: attr.into(),
            uid: None,
            shell: None,
            command: command.into(),
        }
    }
    pub fn shell(mut self, shell: impl Into<String>) -> Self {
        self.shell = Some(shell.into());
        self
    }
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.uid = Some(user.into());
        self
    }
}
#[derive(Debug, Clone)]
pub struct ExecTask {
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
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> Result<TaskStatus>
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
pub struct DryRunExecTask {
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
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> Result<TaskStatus>
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
