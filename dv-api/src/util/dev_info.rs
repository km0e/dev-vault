mod linux;
use std::str::FromStr;

pub use linux::Linux as LinuxOs;
use strum::{AsRefStr, Display, EnumIs};

#[cfg_attr(feature = "rune", derive(rune::Any))]
#[derive(Debug, Hash, Eq, Clone, Copy, Default, Display, EnumIs, AsRefStr, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum Os {
    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
    #[strum(transparent)]
    Linux(LinuxOs),
    #[strum(serialize = "windows")]
    Windows,
    #[strum(serialize = "macos")]
    Mac,
    #[strum(serialize = "unix")]
    Unix,
}

impl Os {
    pub fn compatible(&self, other: &Os) -> bool {
        match other {
            Os::Unknown => true,
            Os::Linux(LinuxOs::Unknown) => matches!(self, Os::Linux(_)),
            Os::Linux(linux) => match self {
                Os::Linux(other_linux) => linux == other_linux,
                _ => false,
            },
            Os::Windows => self == &Os::Windows,
            Os::Mac => self == &Os::Mac,
            Os::Unix => matches!(self, Os::Linux(_) | Os::Unix | Os::Mac),
        }
    }
}

impl From<&str> for Os {
    fn from(s: &str) -> Self {
        if let Ok(os) = LinuxOs::from_str(s) {
            Os::Linux(os)
        } else {
            match s {
                "windows" => Os::Windows,
                "macos" => Os::Mac,
                "unix" => Os::Unix,
                _ => Os::Unknown,
            }
        }
    }
}

impl From<&String> for Os {
    fn from(s: &String) -> Self {
        Os::from(s.as_str())
    }
}

impl From<String> for Os {
    fn from(s: String) -> Self {
        Os::from(s.as_str())
    }
}

#[test]
fn test_os_convert() {
    assert_eq!(Os::Unknown.as_ref(), "unknown");
    assert_eq!(Os::Linux(LinuxOs::Unknown).as_ref(), "linux");
    assert_eq!(Os::from("linux"), Os::Linux(LinuxOs::Unknown));
    assert_eq!(Os::from("manjaro"), Os::Linux(LinuxOs::Manjaro));
}
