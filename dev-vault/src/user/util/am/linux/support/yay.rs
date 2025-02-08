use crate::user::util::am::into_boxed_am;

use super::dev::*;

#[derive(Default)]
pub struct Yay {}

#[async_trait::async_trait]
impl Am for Yay {
    async fn install(&self, dev: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        dev.exec(
            CommandStr::new(
                "yay",
                ["-S", "--noconfirm", "--needed"]
                    .into_iter()
                    .chain(package.iter().map(|p| p.as_str())),
            ),
            None,
        )
        .await
    }
}

into_boxed_am!(Yay);
