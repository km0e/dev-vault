use super::dev::*;

#[derive(Default)]
pub struct Apt {}

impl Apt {
    pub async fn install(&self, user: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        user.exec(
            Command::new(
                "apt",
                ["install", "-y"]
                    .into_iter()
                    .chain(package.iter().map(|p| p.as_str())),
            ),
            None,
        )
        .await
    }
}
