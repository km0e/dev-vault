use super::dev::*;

#[derive(Default)]
pub struct Apt {}

#[async_trait::async_trait]
impl Am for Apt {
    async fn install(
        &self,
        u: &User,
        interactor: &DynInteractor,
        packages: &str,
    ) -> crate::Result<bool> {
        super::install(
            u,
            interactor,
            format!("pkgs=\"{}\";", packages),
            include_str!("sh/apt_query.sh"),
            "apt-get",
            &["install", "-y"][..],
        )
        .await
    }
}
