use super::dev::*;

#[derive(Default)]
pub struct Paru {}

#[async_trait::async_trait]
impl Am for Paru {
    async fn install(
        &self,
        u: &User,
        interactor: &DynInteractor,
        packages: &str,
    ) -> crate::Result<bool> {
        super::install(
            u,
            interactor,
            format!("am=paru;pkgs=\"{}\";", packages),
            include_str!("sh/pacman_query.sh"),
            "paru",
            &["-S", "--noconfirm"][..],
        )
        .await
    }
}
