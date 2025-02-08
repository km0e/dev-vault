use crate::user::util::am::into_boxed_am;

use super::dev::*;

#[derive(Default)]
pub struct Pacman {}

#[async_trait::async_trait]
impl Am for Pacman {
    async fn install(&self, dev: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        dev.exec(
            CommandStr::new(
                "pacman",
                ["-S", "--noconfirm", "--needed"]
                    .into_iter()
                    .chain(package.iter().map(|p| p.as_str())),
            ),
            None,
        )
        .await
    }
}
into_boxed_am!(Pacman);
