use tracing::debug;

use super::dev::*;

pub async fn manjaro_am(u: &BoxedUser) -> crate::Result<BoxedAm> {
    debug!("try to detect manjaro package manager");
    let output = u
        .exec(
            WindowSize::default(),
            Script::Script {
                program: "sh",
                input: Box::new(
                    [r#"stty -echo
                    if command -v yay >/dev/null 2>&1; then
                        exit 1
                    elif command -v paru >/dev/null 2>&1; then
                        exit 2
                    else
                        exit 0
                    fi
                    "#]
                    .into_iter(),
                ),
            },
        )
        .wait()
        .await?;

    if output == 1 {
        debug!("detected yay as package manager");
        Ok(Yay::default().into())
    } else if output == 2 {
        debug!("detected paru as package manager");
        Ok(Paru::default().into())
    } else {
        debug!("detected pacman as package manager");
        Ok(Pacman::default().into())
    }
}
