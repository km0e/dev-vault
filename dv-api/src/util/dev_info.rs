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
        let os = LinuxOs::try_from(value);
        if let Ok(os) = os {
            return Os::Linux(os);
        }
        match value {
            "windows" => Os::Windows,
            "macos" => Os::Mac,
            _ => Os::Unknown,
        }
    }
}
impl From<String> for Os {
    fn from(value: String) -> Self {
        Os::from(value.as_str())
    }
}
