use crate::whatever;

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
        use std::iter::once;
        let args = format!("pkgs=\"{}\";", packages);
        let input = once(args.as_str()).chain(once(include_str!("sh/apt_query.sh")));
        let cmd = Script::Script {
            program: "sh",
            input: Box::new(input),
        };
        let pkgs = u.exec(cmd).output().await?;
        if pkgs.is_empty() {
            return Ok(false);
        }
        let cmd = Script::Split {
            program: "apt-get",
            args: Box::new(["install", "-y"].into_iter().chain(pkgs.split_whitespace())),
        };
        let pp = u.exec(cmd).await?;
        let ec = interactor.ask(pp).await?;
        if ec != 0 {
            whatever!("unexpected exit status {}", ec);
        }
        Ok(true)
    }
}
