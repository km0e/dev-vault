mod linux;
use std::str::FromStr;

pub use linux::Linux as LinuxOs;
use strum::{AsRefStr, Display, EnumIs};

#[cfg_attr(feature = "rune", derive(rune::Any))]
#[derive(Debug, Clone, Copy, Default, Display, EnumIs, AsRefStr, PartialEq)]
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
}

impl rune::alloc::clone::TryClone for Os {
    #[inline]
    fn try_clone(&self) -> rune::alloc::Result<Self> {
        Ok(*self)
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
                _ => Os::Unknown,
            }
        }
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
