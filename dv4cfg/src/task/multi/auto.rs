use super::dev::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTaskConfig {
    pub attr: TaskAttr,
    pub uid: Option<String>,
    pub name: String,
    pub action: String,
}

impl<I: ContextImpl> TaskComplete<I> for AutoTaskConfig {
    fn cast(self, dry_run: bool, target: &Target) -> TaskParts<I> {
        let dst_uid = self.uid.or_else(|| target.dst_uid.clone());
        TaskParts {
            id: self.attr.id,
            target: Target {
                dst_uid,
                ..Default::default()
            },
            next: self.attr.next,
            task: if !dry_run {
                AutoTask::new(self.name, self.action).into()
            } else {
                DryRunAutoTask::new(self.name, self.action).into()
            },
        }
    }
}

impl AutoTaskConfig {
    pub fn new(
        attr: impl Into<TaskAttr>,
        name: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            attr: attr.into(),
            uid: None,
            name: name.into(),
            action: action.into(),
        }
    }
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.uid = Some(user.into());
        self
    }
}
#[derive(Debug, Clone)]
pub struct AutoTask {
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
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> Result<TaskStatus>
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
pub struct DryRunAutoTask {
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
    async fn exec(&self, _target: &Target, context: Arc<Context<I>>) -> Result<TaskStatus>
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
