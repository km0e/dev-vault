use super::dev::*;

#[derive(Default, Debug)]
pub struct Paru {}

impl Paru {
    pub async fn install(
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
