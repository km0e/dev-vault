use std::fmt::Display;

use async_trait::async_trait;
use snafu::OptionExt;

use crate::op::ContextImpl;

#[repr(u8)]
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum TaskStatus {
    #[default]
    DoNothing,
    Success,
    Failed,
}

impl TryFrom<u8> for TaskStatus {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::DoNothing),
            1 => Ok(Self::Success),
            2 => Ok(Self::Failed),
            _ => Err(()),
        }
    }
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TaskStatus::Success => "success",
            TaskStatus::DoNothing => "do nothing",
            TaskStatus::Failed => "failed",
        })
    }
}

#[derive(Debug, Default)]
pub struct Target {
    dst_uid: Option<String>,
    src_uid: Option<String>,
}

impl Target {
    pub fn new(src: Option<impl Into<String>>, dst: Option<impl Into<String>>) -> Self {
        Self {
            src_uid: src.map(|s| s.into()),
            dst_uid: dst.map(|s| s.into()),
        }
    }
    pub fn get_dst_uid(&self) -> crate::Result<&str> {
        self.dst_uid
            .as_deref()
            .with_whatever_context::<_, &str, crate::Error>(|| "no dst_uid")
    }
    pub fn get_uid(&self) -> crate::Result<(&str, &str)> {
        Ok((
            self.src_uid
                .as_deref()
                .with_whatever_context::<_, &str, crate::Error>(|| "no src_uid")?,
            self.get_dst_uid()?,
        ))
    }
    pub fn filter(&self, filter: &mut crate::user::UserFilter) {
        if let Some(uid) = &self.dst_uid {
            filter.insert(uid.to_string());
        }
        if let Some(uid) = &self.src_uid {
            filter.insert(uid.to_string());
        }
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(dst_uid) = &self.dst_uid {
            write!(f, "dst: {}", dst_uid)?;
        }
        if let Some(src_uid) = &self.src_uid {
            write!(f, "src: {}", src_uid)?;
        }
        Ok(())
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

#[async_trait]
pub trait Task<I: ContextImpl> {
    async fn exec(
        &self,
        target: &Target,
        context: std::sync::Arc<crate::op::Context<I>>,
    ) -> crate::Result<TaskStatus>
    where
        I: 'async_trait;
}

pub type BoxedTask<I> = Box<dyn Task<I> + Send + Sync>;

macro_rules! into_boxed_task {
    ($t:ty, $($tail:tt)*) => {
        into_boxed_task!($t);
        into_boxed_task!($($tail)*);
    };
    ($t:ty) => {
        impl<I: ContextImpl> From<$t> for BoxedTask<I> {
            fn from(value: $t) -> Self {
                Box::new(value)
            }
        }
    };
}

pub(crate) use into_boxed_task;

impl<I> std::fmt::Debug for Box<dyn Task<I> + Send + Sync> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Box<dyn Task>").finish()
    }
}

pub trait TaskCast<I: ContextImpl> {
    fn cast(self, dry_run: bool) -> BoxedTask<I>;
}
