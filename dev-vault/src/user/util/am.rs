use std::fmt::Debug;

use async_trait::async_trait;
use mock::MockAm;

mod linux;
mod dev {
    pub use super::super::dev::*;
    pub use super::{Am, BoxedAm};
}
use dev::*;
mod mock;

#[async_trait]
pub trait Am {
    async fn install(&self, u: &User, package: &str) -> crate::Result<BoxedPtyProcess>;
}

pub type BoxedAm = Box<dyn Am + Send + Sync>;
impl Debug for BoxedAm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Am]")
    }
}

macro_rules! into_boxed_am {
    ($t:ty, $($tail:tt)*) => {
        into_boxed_am!($t);
        into_boxed_am!($($tail)*);
    };
    ($t:ty) => {
        impl From<$t> for BoxedAm {
            fn from(value: $t) -> Self {
                Box::new(value)
            }
        }
    };
}
pub(crate) use into_boxed_am;

pub async fn new_am(u: &BoxedUser, os: &str) -> crate::Result<BoxedAm> {
    #[cfg(target_os = "linux")]
    Ok(linux::try_match(u, os)
        .await?
        .unwrap_or_else(|| MockAm {}.into()))
}
