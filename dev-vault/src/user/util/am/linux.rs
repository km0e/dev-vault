use super::dev::{self, *};

mod apk;
mod apt;
mod distro;
mod pacman;

pub fn try_match(os: &str) -> Option<BoxedAm> {
    match os {
        "manjaro" => Some(distro::Manjaro::default().into()),
        "alpine" => Some(distro::Alpine::default().into()),
        _ => None,
    }
}
