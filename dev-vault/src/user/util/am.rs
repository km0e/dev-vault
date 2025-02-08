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
    async fn install(&self, u: &User, package: &[String]) -> crate::Result<BoxedPtyProcess>;
}

pub type BoxedAm = Box<dyn Am + Send + Sync>;

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

pub async fn new_am(u: &BoxedUser, e: &Environment) -> crate::Result<BoxedAm> {
    let Some(os) = e.os.as_ref() else {
        return Ok(MockAm {}.into());
    };
    #[cfg(target_os = "linux")]
    Ok(linux::try_match(u, os)
        .await?
        .unwrap_or_else(|| MockAm {}.into()))
}
