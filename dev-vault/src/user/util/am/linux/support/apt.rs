use crate::user::util::am::into_boxed_am;

use super::dev::*;

#[derive(Default)]
pub struct Apt {}

#[async_trait::async_trait]
impl Am for Apt {
    async fn install(&self, u: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        u.exec(
            CommandStr::new(
                "apt",
                ["install", "-y"]
                    .into_iter()
                    .chain(package.iter().map(String::as_str)),
            ),
            None,
        )
        .await
    }
}

into_boxed_am!(Apt);
