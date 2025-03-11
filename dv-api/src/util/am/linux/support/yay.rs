use super::dev::*;

#[derive(Default)]
pub struct Yay {}

#[async_trait::async_trait]
impl Am for Yay {
    async fn install(
        &self,
        u: &User,
        interactor: &DynInteractor,
        packages: &str,
    ) -> crate::Result<bool> {
        super::install(
            u,
            interactor,
            format!("am=yay;pkgs=\"{}\";", packages),
            include_str!("sh/pacman_query.sh"),
            "yay",
            &["-S", "--noconfirm"][..],
        )
        .await
    }
}
