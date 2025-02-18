#[derive(Debug, Clone)]
pub enum Linux {
    Unknown,
    Manjaro,
    Alpine,
    Debian,
}

impl TryFrom<&str> for Linux {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "linux" => Linux::Unknown,
            "manjaro" => Linux::Manjaro,
            "alpine" => Linux::Alpine,
            "debian" => Linux::Debian,
            _ => return Err(()),
        })
    }
}
