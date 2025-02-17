use rustix::path::Arg;
use snafu::whatever;

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
        use std::iter::once;
        let args = format!("am=paru;pkgs=\"{}\";", packages);
        let input = once(args.as_str()).chain(once(include_str!("sh/pacman_query.sh")));
        let cmd = Script::Script {
            program: "sh",
            input: Box::new(input),
        };
        let pkgs = u.exec(cmd).await?.output().await?;
        if pkgs.is_empty() {
            return Ok(false);
        }
        let pkgs = pkgs.to_string_lossy();
        let cmd = Script::Split {
            program: "paru",
            args: Box::new(
                ["-S", "--noconfirm"]
                    .into_iter()
                    .chain(pkgs.split_whitespace()),
            ),
        };
        let mut pp = u.exec(cmd).await?;
        let ec = interactor.ask(&mut pp).await?;
        if ec != 0 {
            whatever!("unexpected exit status {}", ec);
        }
        Ok(true)
    }
}
