use dev_vault::task;
use serde::{Deserialize, Serialize};

use crate::adapter::TaskParts;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Target {
    pub src_uid: Option<String>,
    pub dst_uid: Option<String>,
}

impl Target {
    pub fn with_dst(mut self, dst: impl Into<String>) -> Self {
        self.dst_uid = Some(dst.into());
        self
    }
    pub fn with_src(mut self, src: impl Into<String>) -> Self {
        self.src_uid = Some(src.into());
        self
    }
    pub fn cast(&self) -> task::Target {
        task::Target::new(self.src_uid.clone(), self.dst_uid.clone())
    }
}

impl std::ops::ShlAssign<&Target> for Target {
    fn shl_assign(&mut self, rhs: &Target) {
        self.dst_uid = self.dst_uid.take().or_else(|| rhs.dst_uid.clone());
        self.src_uid = self.src_uid.take().or_else(|| rhs.src_uid.clone());
    }
}

impl std::ops::ShlAssign<Target> for Target {
    fn shl_assign(&mut self, rhs: Target) {
        self.dst_uid = self.dst_uid.take().or(rhs.dst_uid);
        self.src_uid = self.src_uid.take().or(rhs.src_uid);
    }
}

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
