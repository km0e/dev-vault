use super::dev::*;

pub async fn debian_am(_: &BoxedUser) -> crate::Result<BoxedAm> {
    Ok(Apt::default().into())
}
