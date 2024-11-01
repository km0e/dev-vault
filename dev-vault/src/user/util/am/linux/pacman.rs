use super::dev::*;

#[derive(Default)]
pub struct Pacman {}

impl Pacman {
    pub async fn install(&self, dev: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        dev.exec(
            Command::new(
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
