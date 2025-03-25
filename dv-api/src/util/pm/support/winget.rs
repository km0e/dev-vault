use super::dev::*;

pub async fn install(u: &User, interactor: &DynInteractor, packages: &str) -> Result<bool> {
    use std::iter::once;
    let args = format!("$pkgs = \"{}\";", packages);
    let input = once(args.as_str()).chain(once(include_str!("sh/winget_query.ps1")));
    let cmd = Script::powershell(Box::new(input));
    let pkgs = u.exec(cmd).output().await?;
    let pkgs = pkgs.trim();
    if pkgs.is_empty() {
        return Ok(false);
    }
    let args = format!("$pkgs = \"{}\";", pkgs);
    let input = once(args.as_str()).chain(once(include_str!("sh/winget_install.ps1")));
    let cmd = Script::powershell(Box::new(input));
    let pp = u.pty(cmd, interactor.window_size().await).await?;
    let ec = interactor.ask(pp).await?;
    if ec != 0 {
        whatever!("unexpected exit status {}", ec);
    }
    Ok(true)
}
