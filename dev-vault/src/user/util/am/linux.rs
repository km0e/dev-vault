use super::dev::*;
mod dev {
    pub use super::super::dev::*;
    pub use super::support::*;
}

mod alpine;
mod debian;
mod manjaro;
mod support;
pub async fn try_match(u: &BoxedUser, os: &str) -> crate::Result<Option<BoxedAm>> {
    Ok(match os {
        "manjaro" => Some(manjaro::manjaro_am(u).await?),
        "alpine" => Some(alpine::alpine_am(u).await?),
        "debian" => Some(debian::debian_am(u).await?),
        _ => None,
    })
}
