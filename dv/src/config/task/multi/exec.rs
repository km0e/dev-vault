use dev_vault::{
    op::ContextImpl,
    task::{self, TaskCast},
};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::adapter::TaskParts;

use super::core::*;

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
            task: task::ExecTaskConfig {
                shell: self.shell,
                command: self.command,
            }
            .cast(dry_run),
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
