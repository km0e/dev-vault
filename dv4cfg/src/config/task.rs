use serde::{Deserialize, Serialize};

use crate::{adapter::TaskParts, task::Target};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAttr {
    pub id: String,
    #[serde(default)]
    pub next: Vec<String>,
}

impl From<&str> for TaskAttr {
    fn from(value: &str) -> Self {
        Self {
            id: value.into(),
            next: Vec::default(),
        }
    }
}
impl TaskAttr {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            next: Vec::default(),
        }
    }
    pub fn with_next(mut self, next: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.next = next.into_iter().map(|s| s.into()).collect();
        self
    }
}

pub trait TaskComplete<I> {
    fn cast(self, dry_run: bool, target: &Target) -> TaskParts<I>;
}



