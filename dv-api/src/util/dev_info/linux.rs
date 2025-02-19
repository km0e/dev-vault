use strum::{AsRefStr, Display, EnumString};

#[derive(Default, Debug, Clone, AsRefStr, Display, EnumString, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum Linux {
    #[default]
    #[strum(serialize = "linux")]
    Unknown,
    #[strum(serialize = "manjaro")]
    Manjaro,
    #[strum(serialize = "alpine")]
    Alpine,
    #[strum(serialize = "debian")]
    Debian,
}
