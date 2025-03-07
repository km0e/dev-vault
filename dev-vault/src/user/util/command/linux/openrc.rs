use super::dev::*;

#[derive(Default)]
pub struct Openrc {}

impl Openrc {
    pub async fn setup<U: UserImpl>(&self, user: &U, name: &str) -> crate::Result<BoxedPtyProcess> {
        user.exec(["rc-update", "add", name, "default"].as_ref().into(), None)
            .await
    }
    pub async fn reload<U: UserImpl>(
        &self,
        user: &U,
        name: &str,
    ) -> crate::Result<BoxedPtyProcess> {
        user.exec(["rc-service", name, "restart"].as_ref().into(), None)
            .await
    }
}
