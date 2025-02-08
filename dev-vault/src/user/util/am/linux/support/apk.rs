use crate::user::util::am::into_boxed_am;

use super::dev::*;

#[derive(Default)]
pub struct Apk {}

#[async_trait::async_trait]
impl Am for Apk {
    async fn install(&self, u: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        u.exec(
            CommandStr::new(
                "apk",
                std::iter::once("add").chain(package.iter().map(String::as_str)),
            ),
            None,
        )
        .await
    }
}

into_boxed_am!(Apk);
