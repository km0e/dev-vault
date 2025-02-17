use rustix::path::Arg;
use snafu::whatever;
use tracing::info;

use super::dev::*;

#[derive(Default)]
pub struct Pacman {}

#[async_trait::async_trait]
impl Am for Pacman {
    async fn install(
        &self,
        u: &User,
        interactor: &DynInteractor,
        packages: &str,
    ) -> crate::Result<bool> {
        use std::iter::once;
        let args = format!("am=pacman;pkgs=\"{}\";", packages);
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
        info!("pkgs[{}] need to be installed", pkgs);
        let cmd = Script::Split {
            program: "pacman",
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
