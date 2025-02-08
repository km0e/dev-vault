use crate::user::util::am::into_boxed_am;

use super::dev::*;

#[derive(Default)]
pub struct Paru {}

#[async_trait::async_trait]
impl Am for Paru {
    async fn install(&self, u: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        u.exec(
            CommandStr::new(
                "paru",
                ["-S", "--noconfirm", "--needed"]
                    .into_iter()
                    .chain(package.iter().map(String::as_str)),
            ),
            None,
        )
        .await
    }
}

into_boxed_am!(Paru);
