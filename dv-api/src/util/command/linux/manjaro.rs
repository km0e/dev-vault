use super::dev::*;

#[derive(Default)]
pub struct Manjaro {
    systemd: super::Systemd,
}

#[async_trait]
impl<U: UserImpl + Send + Sync> CommandUtil<U> for Manjaro {
    async fn setup(&self, user: &U, name: &str) -> Result<i32> {
        self.systemd.setup(user, name).await
    }
    async fn reload(&self, user: &U, name: &str) -> Result<i32> {
        self.systemd.reload(user, name).await
    }
    //file utils
    async fn copy(&self, dev: &U, src: &str, dst: &str) -> Result<i32> {
        dev.exec(["cp", src, dst].as_ref().into())
            .await?
            .wait()
            .await
    }
}
