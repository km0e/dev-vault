use crate::whatever;
use tracing::info;

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
        use std::iter::once;
        let args = format!("am=yay;pkgs=\"{}\";", packages);
        let input = once(args.as_str()).chain(once(include_str!("sh/pacman_query.sh")));
        let cmd = Script::Script {
            program: "sh",
            input: Box::new(input),
        };
        let pkgs = u.exec(cmd).output().await?;
        if pkgs.is_empty() {
            return Ok(false);
        }
        info!("pkgs[{}] need to be installed", pkgs);
        let cmd = Script::Split {
            program: "yay",
            args: Box::new(
                ["-S", "--noconfirm"]
                    .into_iter()
                    .chain(pkgs.split_whitespace()),
            ),
        };
        let pp = u.exec(cmd).await?;
        let ec = interactor.ask(pp).await?;
        if ec != 0 {
            whatever!("unexpected exit status {}", ec);
        }
        Ok(true)
    }
}
