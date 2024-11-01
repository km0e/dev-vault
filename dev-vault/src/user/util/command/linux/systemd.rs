use super::dev::*;

#[derive(Default)]
pub struct Systemd {}

impl Systemd {
    pub async fn setup<U: UserImpl>(&self, user: &U, name: &str) -> crate::Result<BoxedPtyProcess> {
        user.exec(["systemctl", "enable", name].as_ref().into(), None)
            .await
    }
    pub async fn reload<U: UserImpl>(
        &self,
        user: &U,
        name: &str,
    ) -> crate::Result<BoxedPtyProcess> {
        user.exec(
            ["systemctl", "reload-or-restart", name].as_ref().into(),
            None,
        )
        .await
    }
}
