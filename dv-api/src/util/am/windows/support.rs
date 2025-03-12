use tracing::debug;

use crate::util::am::into_boxed_am;

use super::dev::*;
#[derive(Default)]
pub struct WinGet {}

#[async_trait::async_trait]
impl Am for WinGet {
    async fn install(
        &self,
        u: &User,
        interactor: &DynInteractor,
        packages: &str,
    ) -> crate::Result<bool> {
        debug!("installing {}", packages);
        use std::iter::once;
        let args = format!("$pkgs = \"{}\";", packages);
        let input = once(args.as_str()).chain(once(include_str!("sh/winget_query.ps1")));
        let cmd = Script::powershell(Box::new(input));
        let pkgs = u.exec(cmd).output().await?;
        let pkgs = pkgs.trim();
        if pkgs.is_empty() {
            debug!("no package need to be installed");
            return Ok(false);
        }
        let args = format!("$pkgs = \"{}\";", pkgs);
        debug!("installing {}", pkgs);
        let input = once(args.as_str()).chain(once(include_str!("sh/winget_install.ps1")));
        let cmd = Script::powershell(Box::new(input));
        let pp = u.pty(cmd, WindowSize::default()).await?;
        let ec = interactor.ask(pp).await?;
        if ec != 0 {
            whatever!("unexpected exit status {}", ec);
        }
        Ok(true)
    }
}

into_boxed_am!(WinGet);
