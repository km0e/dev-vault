use std::fmt::Display;

use strum::EnumIs;
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

#[derive(Debug, EnumIs)]
pub enum Pm {
    Apk(Apk),
    Apt(Apt),
    Pacman(Pacman),
    Yay(Yay),
    Paru(Paru),
    WinGet(WinGet),
    Unknown,
}

#[cfg_attr(feature = "rune", derive(rune::Any))]
#[derive(Debug, Default)]
pub struct Package {
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub apk: Option<String>,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub apt: Option<String>,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub pacman: Option<String>,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub yay: Option<String>,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub paru: Option<String>,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub winget: Option<String>,
}

impl Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(apk) = &self.apk {
            write!(f, "apk:{} ", apk)?;
        }
        if let Some(apt) = &self.apt {
            write!(f, "apt:{} ", apt)?;
        }
        if let Some(pacman) = &self.pacman {
            write!(f, "pacman:{} ", pacman)?;
        }
        if let Some(yay) = &self.yay {
            write!(f, "yay:{} ", yay)?;
        }
        if let Some(paru) = &self.paru {
            write!(f, "paru:{} ", paru)?;
        }
        if let Some(winget) = &self.winget {
            write!(f, "winget:{} ", winget)?;
        }
        Ok(())
    }
}

impl Pm {
    pub async fn new(u: &BoxedUser, os: &Os) -> crate::Result<Self> {
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

macro_rules! from {
    ($($x:ident),*) => {
        $(
            impl From<$x> for Pm {
                fn from(p: $x) -> Self {
                    Self::$x(p)
                }
            }
        )*
    };
}

from!(Apk, Apt, Pacman, Yay, Paru, WinGet);

impl Pm {
    pub async fn install(
        &self,
        u: &User,
        interactor: &DynInteractor,
        package: &Package,
    ) -> crate::Result<bool> {
        match (self, package) {
            (
                Self::Apk(a),
                Package {
                    apk: Some(package), ..
                },
            ) => a.install(u, interactor, package).await,
            (
                Self::Apt(a),
                Package {
                    apt: Some(package), ..
                },
            ) => a.install(u, interactor, package).await,
            (
                Self::Pacman(a),
                Package {
                    pacman: Some(package),
                    ..
                },
            ) => a.install(u, interactor, package).await,
            (
                Self::Yay(a),
                Package {
                    yay: Some(package), ..
                },
            ) => a.install(u, interactor, package).await,
            (
                Self::Paru(a),
                Package {
                    paru: Some(package),
                    ..
                },
            ) => a.install(u, interactor, package).await,
            (
                Self::WinGet(a),
                Package {
                    winget: Some(package),
                    ..
                },
            ) => a.install(u, interactor, package).await,
            (Self::Unknown, _) => whatever!("Unknown Pm"),
            _ => whatever!("nothing matched"),
        }
    }
}
