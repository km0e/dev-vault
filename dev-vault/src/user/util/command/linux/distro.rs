use crate::user::util::command::into_boxed_command_util;

use super::dev::*;
use async_trait::async_trait;

use super::openrc::Openrc;

use super::systemd::Systemd;

#[derive(Default)]
pub struct Debian {
    systemd: Systemd,
}

#[async_trait]
impl<U: UserImpl + Send + Sync> CommandUtil<U> for Debian {
    async fn setup(&self, user: &U, name: &str) -> crate::Result<BoxedPtyProcess> {
        self.systemd.setup(user, name).await
    }
    async fn reload(&self, user: &U, name: &str) -> crate::Result<BoxedPtyProcess> {
        self.systemd.reload(user, name).await
    }
}
#[derive(Default)]
pub struct Manjaro {
    systemd: Systemd,
}

#[async_trait]
impl<U: UserImpl + Send + Sync> CommandUtil<U> for Manjaro {
    async fn setup(&self, user: &U, name: &str) -> crate::Result<BoxedPtyProcess> {
        self.systemd.setup(user, name).await
    }
    async fn reload(&self, user: &U, name: &str) -> crate::Result<BoxedPtyProcess> {
        self.systemd.reload(user, name).await
    }
    //file utils
    async fn copy(&self, dev: &U, src: &str, dst: &str) -> crate::Result<BoxedPtyProcess> {
        dev.exec(["cp", src, dst].as_ref().into()).await
    }
}

#[derive(Default)]
pub struct Alpine {
    openrc: Openrc,
}

#[async_trait]
impl<U: UserImpl + Send + Sync> CommandUtil<U> for Alpine {
    async fn setup(&self, user: &U, name: &str) -> crate::Result<BoxedPtyProcess> {
        self.openrc.setup(user, name).await
    }
    async fn reload(&self, user: &U, name: &str) -> crate::Result<BoxedPtyProcess> {
        self.openrc.reload(user, name).await
    }
}

into_boxed_command_util!(Debian, Manjaro, Alpine);
