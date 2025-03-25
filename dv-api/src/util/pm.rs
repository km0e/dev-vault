use std::collections::HashMap;

use strum::{Display, EnumIs, EnumString};
mod dev {
    pub use super::super::dev::*;
    pub use super::super::dev_info::*;
    pub use super::Pm;
    pub use super::support::*;
    pub use crate::{User, process::DynInteractor, whatever};
    pub use e4pty::prelude::*;
}
use dev::*;
use tracing::info;
mod platform;
mod support;
use super::Os;

#[derive(Debug, Clone, Copy, Display, Default, Hash, PartialEq, Eq, EnumIs, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Pm {
    Apk,
    Apt,
    Pacman,
    Yay,
    Paru,
    WinGet,
    #[default]
    Unknown,
}

#[derive(Debug, Default)]
pub struct Package<'a> {
    pub pm: HashMap<Pm, &'a str>,
}

impl Package<'_> {
    pub async fn install(&self, u: &User, interactor: &DynInteractor, pm: &Pm) -> Result<bool> {
        if let Some(package) = self.pm.get(pm) {
            match pm {
                Pm::Apk => apk::install(u, interactor, package).await,
                Pm::Apt => apt::install(u, interactor, package).await,
                Pm::Pacman => pacman::install(u, interactor, package).await,
                Pm::Yay => yay::install(u, interactor, package).await,
                Pm::Paru => paru::install(u, interactor, package).await,
                Pm::WinGet => winget::install(u, interactor, package).await,
                Pm::Unknown => whatever!("Unknown Pm"),
            }
        } else {
            whatever!("No package found for {:?}", pm)
        }
    }
}

impl Pm {
    pub async fn new(u: &BoxedUser, os: &Os) -> Result<Self> {
        info!("new_am os:{:?}", os);
        match os {
            Os::Linux(os) => match os {
                LinuxOs::Manjaro => platform::manjaro::detect(u).await,
                LinuxOs::Debian => platform::debian::detect(u).await,
                LinuxOs::Alpine => platform::alpine::detect(u).await,
                LinuxOs::Unknown => whatever!("Unknown LinuxOs"),
            },
            Os::Windows => platform::windows::detect(u).await,
            _ => Ok(Self::Unknown),
        }
    }
}
