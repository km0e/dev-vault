use super::dev::*;

#[derive(Default, Debug)]
pub struct Apt {}

impl Apt {
    pub async fn install(
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
