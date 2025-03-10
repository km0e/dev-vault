use super::dev::*;
mod dev {
    pub use super::super::dev::*;
}

mod support;

pub async fn try_match(_: &BoxedUser) -> crate::Result<Option<BoxedAm>> {
    Ok(Some(support::WinGet {}.into()))
}
