use tracing::debug;

use super::dev::*;

pub async fn manjaro_am(u: &BoxedUser) -> crate::Result<BoxedAm> {
    debug!("try to detact manjaro package manager");
    let output = u
        .exec(
            CommandStr::Whole(
                r#"if command -v yay >/dev/null 2>&1; then
                 echo "yay"
               elif command -v paru >/dev/null 2>&1; then
                 echo "paru"
               else
                 echo "pacman"
               fi
               exit 0"#,
            ),
            Some("sh"),
        )
        .await?
        .output()
        .await?;
    if output.starts_with(b"yay") {
        Ok(Yay::default().into())
    } else if output.starts_with(b"paru") {
        Ok(Paru::default().into())
    } else {
        Ok(Pacman::default().into())
    }
}
