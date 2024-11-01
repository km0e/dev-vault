use super::dev::{self, *};

mod distro;
mod openrc;
mod systemd;
pub fn try_match<U: UserImpl + Send + Sync>(os: &str) -> Option<BoxedCommandUtil<U>> {
    match os {
        "manjaro" => Some(distro::Manjaro::default().into()),
        "debian" => Some(distro::Debian::default().into()),
        "alpine" => Some(distro::Alpine::default().into()),
        _ => None,
    }
}
