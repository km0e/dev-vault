use dev_vault::op::ContextImpl;
use serde::{Deserialize, Serialize};

use crate::adapter::GroupParts;

use super::task::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cite {
    pub attr: TaskAttr,
    #[serde(default)]
    pub target: Target,
}

impl Cite {
    pub fn new(attr: impl Into<TaskAttr>) -> Self {
        Self {
            attr: attr.into(),
            target: Default::default(),
        }
    }
    pub fn with_target(mut self, target: Target) -> Self {
        self.target = target;
        self
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TaskGroupConfig {
    pub id: String,
    #[serde(default)]
    pub target: Target,
    #[serde(default)]
    pub cites: Vec<Cite>,
    #[serde(default)]
    pub auto: Vec<AutoTaskConfig>,
    #[serde(default)]
    pub copy: Vec<CopyTaskConfig>,
    #[serde(default)]
    pub app: Vec<AppTaskConfig>,
    #[serde(default)]
    pub exec: Vec<ExecTaskConfig>,
}
impl TaskGroupConfig {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }
    pub fn with_target(mut self, target: Target) -> Self {
        self.target = target;
        self
    }
    pub fn with_cites(mut self, cites: impl IntoIterator<Item = Cite>) -> Self {
        self.cites = cites.into_iter().collect();
        self
    }
    pub fn with_auto(mut self, auto: impl IntoIterator<Item = AutoTaskConfig>) -> Self {
        self.auto = auto.into_iter().collect();
        self
    }
    pub fn with_copy(mut self, copy: impl IntoIterator<Item = CopyTaskConfig>) -> Self {
        self.copy = copy.into_iter().collect();
        self
    }
    pub fn with_app(mut self, app: impl IntoIterator<Item = AppTaskConfig>) -> Self {
        self.app = app.into_iter().collect();
        self
    }
    pub fn with_exec(mut self, exec: impl IntoIterator<Item = ExecTaskConfig>) -> Self {
        self.exec = exec.into_iter().collect();
        self
    }
    pub fn cast<I: ContextImpl>(self, dry_run: bool) -> GroupParts<I> {
        let mut pp = GroupParts::new(self.id);
        for task in self.auto {
            pp.tasks.push(task.cast(dry_run, &self.target));
        }
        for task in self.copy {
            let t = task.cast(dry_run, &self.target);
            pp.tasks.push(t);
        }
        for task in self.app {
            pp.tasks.push(task.cast(dry_run, &self.target));
        }
        for task in self.exec {
            pp.tasks.push(task.cast(dry_run, &self.target));
        }
        pp.cites.extend(self.cites.into_iter().map(|mut cite| {
            cite.target <<= &self.target;
            cite
        }));
        pp
    }
}
