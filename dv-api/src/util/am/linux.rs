use snafu::whatever;

use crate::util::dev_info::LinuxOs;

use super::dev::*;
mod dev {
    pub use super::super::dev::*;
    pub use super::support::*;
}

mod alpine;
mod debian;
mod manjaro;
mod support;
pub async fn try_match(u: &BoxedUser, os: &LinuxOs) -> crate::Result<Option<BoxedAm>> {
    Ok(match os {
        LinuxOs::Manjaro => Some(manjaro::manjaro_am(u).await?),
        LinuxOs::Debian => Some(debian::debian_am(u).await?),
        LinuxOs::Alpine => Some(alpine::alpine_am(u).await?),
        LinuxOs::Unknown => whatever!("Unknown LinuxOs"),
    })
}
