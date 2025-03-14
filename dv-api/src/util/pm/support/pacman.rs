use super::dev::*;

#[derive(Default, Debug)]
pub struct Pacman {}

impl Pacman {
    pub async fn install(
        &self,
        u: &User,
        interactor: &DynInteractor,
        packages: &str,
    ) -> crate::Result<bool> {
        super::install(
            u,
            interactor,
            format!("am=pacman;pkgs=\"{}\";", packages),
            include_str!("sh/pacman_query.sh"),
            "pacman",
            &["-S", "--noconfirm"][..],
        )
        .await
    }
}
