use super::dev::*;

pub async fn alpine_am(_: &BoxedUser) -> crate::Result<BoxedAm> {
    Ok(Apk::default().into())
}
