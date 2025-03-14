use super::dev::*;

pub async fn detect(_: &BoxedUser) -> crate::Result<Pm> {
    Ok(Apt::default().into())
}
