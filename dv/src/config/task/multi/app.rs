use dev_vault::{
    op::ContextImpl,
    task::{self, TaskCast},
};
use serde::{Deserialize, Serialize};

use crate::adapter::TaskParts;

use super::core::*;

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
            task: task::AppTaskConfig { pkgs: self.pkgs }.cast(dry_run),
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
