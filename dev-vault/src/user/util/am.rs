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
    async fn install(&self, dev: &User, package: &[String]) -> crate::Result<BoxedPtyProcess>;
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

impl From<&Environment> for BoxedAm {
    fn from(value: &Environment) -> Self {
        let Some(os) = value.os.as_ref() else {
            return MockAm {}.into();
        };
        linux::try_match(os).unwrap_or_else(|| MockAm {}.into())
    }
}
