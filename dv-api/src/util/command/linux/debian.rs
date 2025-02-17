use super::dev::*;

#[derive(Default)]
pub struct Debian {
    systemd: super::Systemd,
}

#[async_trait]
impl<U: UserImpl + Send + Sync> CommandUtil<U> for Debian {
    async fn setup(&self, user: &U, name: &str) -> Result<i32> {
        self.systemd.setup(user, name).await
    }
    async fn reload(&self, user: &U, name: &str) -> Result<i32> {
        self.systemd.reload(user, name).await
    }
}
