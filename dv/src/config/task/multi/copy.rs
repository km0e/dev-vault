use dev_vault::{
    op::ContextImpl,
    task::{self, TaskCast},
};
use serde::{Deserialize, Serialize};

use crate::adapter::TaskParts;

use super::core::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyTaskConfig {
    pub attr: TaskAttr,
    #[serde(default)]
    pub target: Target,
    pub pair: Vec<(String, String)>,
}

impl<I: ContextImpl> TaskComplete<I> for CopyTaskConfig {
    fn cast(self, dry_run: bool, target: &Target) -> TaskParts<I> {
        let src_uid = self.target.src_uid.or_else(|| target.src_uid.clone());
        let dst_uid = self.target.dst_uid.or_else(|| target.dst_uid.clone());
        TaskParts {
            id: self.attr.id,
            target: Target { src_uid, dst_uid },
            next: self.attr.next,
            task: task::CopyTaskConfig { pair: self.pair }.cast(dry_run),
        }
    }
}

impl CopyTaskConfig {
    pub fn new(
        attr: impl Into<TaskAttr>,
        pair: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            attr: attr.into(),
            target: Target::default(),
            pair: pair
                .into_iter()
                .map(|(src, dst)| (src.into(), dst.into()))
                .collect(),
        }
    }
    pub fn with_src(mut self, uid: impl Into<String>) -> Self {
        self.target.src_uid = Some(uid.into());
        self
    }
    pub fn with_dst(mut self, uid: impl Into<String>) -> Self {
        self.target.dst_uid = Some(uid.into());
        self
    }
}
