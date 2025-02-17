use super::{
    dev::{self, *},
    into_boxed_command_util,
};

mod alpine;
pub use alpine::Alpine;
mod debian;
pub use debian::Debian;
mod manjaro;
pub use manjaro::Manjaro;
mod support;
use support::*;

pub fn try_match<U: UserImpl + Send + Sync>(os: &str) -> Option<BoxedCommandUtil<U>> {
    match os {
        "manjaro" => Some(Manjaro::default().into()),
        "debian" => Some(Debian::default().into()),
        "alpine" => Some(Alpine::default().into()),
        _ => Some(Linux::default().into()),
    }
}
#[derive(Default)]
pub struct Linux {
    systemd: support::Systemd,
}

#[async_trait]
impl<U: UserImpl + Send + Sync> CommandUtil<U> for Linux {
    async fn setup(&self, user: &U, name: &str) -> crate::Result<i32> {
        self.systemd.setup(user, name).await
    }
    async fn reload(&self, user: &U, name: &str) -> crate::Result<i32> {
        self.systemd.reload(user, name).await
    }
}

into_boxed_command_util!(Linux, Alpine, Debian, Manjaro);
