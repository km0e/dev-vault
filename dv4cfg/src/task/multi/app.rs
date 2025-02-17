use super::dev::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTaskConfig {
    pub attr: TaskAttr,
    pub uid: Option<String>,
    pub pkgs: Vec<String>,
}

impl<I: ContextImpl> TaskComplete<I> for AppTaskConfig {
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
                AppTask::new(self.pkgs).into()
            } else {
                DryRunAppTask::new(self.pkgs).into()
            },
        }
    }
}

impl AppTaskConfig {
    pub fn new(
        attr: impl Into<TaskAttr>,
        pkgs: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            attr: attr.into(),
            uid: None,
            pkgs: pkgs.into_iter().map(|s| s.into()).collect(),
        }
    }
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.uid = Some(user.into());
        self
    }
}
#[derive(Debug, Clone)]
pub struct AppTask {
    pub pkgs: String,
}

impl AppTask {
    pub fn new(pkgs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            pkgs: pkgs
                .into_iter()
                .map(|n| n.into())
                .fold(String::new(), |mut acc, n| {
                    if !acc.is_empty() {
                        acc.push(' ');
                    }
                    acc.push_str(&n);
                    acc
                }),
        }
    }
}

#[async_trait]
impl<I: ContextImpl> Task<I> for AppTask {
    async fn exec(&self, target: &Target, context: Arc<Context<I>>) -> Result<TaskStatus>
    where
        I: 'async_trait,
    {
        let uid = target.get_dst_uid()?;
        let user = context.get_user(uid, false)?;
        let int = context.get_interactor();
        let res = user.app(int, &self.pkgs).await?;
        Ok(if res {
            TaskStatus::Success
        } else {
            TaskStatus::Failed
        })
    }
}

#[derive(Debug, Clone)]
pub struct DryRunAppTask {
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
    async fn exec(&self, _target: &Target, context: Arc<Context<I>>) -> Result<TaskStatus>
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
