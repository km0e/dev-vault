use async_trait::async_trait;

use crate::user::util::am::into_boxed_am;

use super::super::Am;
use super::apk::Apk;
use super::apt::Apt;
use super::dev::*;
use super::pacman::Pacman;

#[derive(Default)]
pub struct Manjaro {
    pacman: Pacman,
}

#[async_trait]
impl Am for Manjaro {
    async fn install(&self, dev: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        self.pacman.install(dev, package).await
    }
}

#[derive(Default)]
pub struct Debian {
    apt: Apt,
}

#[async_trait]
impl Am for Debian {
    async fn install(&self, dev: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        self.apt.install(dev, package).await
    }
}

#[derive(Default)]
pub struct Alpine {
    apk: Apk,
}

#[async_trait]
impl Am for Alpine {
    async fn install(&self, dev: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        self.apk.install(dev, package).await
    }
}

into_boxed_am!(Manjaro, Debian, Alpine);
