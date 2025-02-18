mod linux;
pub use linux::Linux as LinuxOs;
use strum_macros::{Display, EnumIs};
#[derive(Debug, Clone, Default, EnumIs, Display)]
pub enum Os {
    #[default]
    Unknown,
    Linux(LinuxOs),
    Windows,
    Mac,
}

impl From<&str> for Os {
    fn from(value: &str) -> Self {
        let os = LinuxOs::from(value);

        match os {
            LinuxOs::Unknown => match value {
                "windows" => Os::Windows,
                "macos" => Os::Mac,
                _ => Os::Unknown,
            },
            _ => Os::Linux(os),
        }
    }
}
