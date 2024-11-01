use dev_vault::{
    op::ContextImpl,
    task::{self, TaskCast},
};
use serde::{Deserialize, Serialize};

use crate::adapter::TaskParts;

use super::core::*;

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
            task: task::AutoTaskConfig {
                name: self.name,
                action: self.action,
            }
            .cast(dry_run),
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
