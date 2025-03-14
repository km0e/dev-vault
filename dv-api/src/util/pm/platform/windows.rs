use super::dev::*;

pub async fn detect(_u: &BoxedUser) -> crate::Result<Pm> {
    Ok(WinGet::default().into())
}
