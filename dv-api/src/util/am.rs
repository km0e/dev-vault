use std::fmt::Debug;

use mock::MockAm;

mod linux;
mod dev {
    pub use super::super::dev::*;
    pub use super::{Am, BoxedAm};
    pub use crate::{User, process::DynInteractor};
    pub use e4pty::Script;
}
use dev::*;
mod mock;

#[async_trait]
pub trait Am {
    async fn install(
        &self,
        u: &User,
        interactor: &DynInteractor,
        package: &str,
    ) -> crate::Result<bool>;
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

use super::Os;

pub async fn new_am(u: &BoxedUser, os: &Os) -> crate::Result<BoxedAm> {
    match os {
        Os::Linux(os) => linux::try_match(u, os)
            .await
            .map(|x| x.unwrap_or_else(|| MockAm {}.into())),
        _ => Ok(MockAm {}.into()),
    }
}
