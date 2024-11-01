use super::dev::*;

#[derive(Default)]
pub struct Apk {}

impl Apk {
    pub async fn install(&self, user: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        user.exec(
            Command::new(
                "apk",
                std::iter::once("add").chain(package.iter().map(|p| p.as_str())),
            ),
            None,
        )
        .await
    }
}
