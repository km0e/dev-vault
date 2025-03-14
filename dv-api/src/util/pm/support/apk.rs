use super::dev::*;

#[derive(Default, Debug)]
pub struct Apk {}

impl Apk {
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
            include_str!("sh/apk_query.sh"),
            "apk",
            &["add"][..],
        )
        .await
    }
}
