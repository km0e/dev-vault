#[derive(Debug, Clone)]
pub enum Linux {
    Unknown,
    Manjaro,
    Alpine,
    Debian,
}

impl From<&str> for Linux {
    fn from(value: &str) -> Self {
        match value {
            "manjaro" => Linux::Manjaro,
            "alpine" => Linux::Alpine,
            "debian" => Linux::Debian,
            _ => Linux::Unknown,
        }
    }
}
