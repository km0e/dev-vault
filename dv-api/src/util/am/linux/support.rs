use super::dev::{self, *};
use crate::util::am::into_boxed_am;

mod apk;
pub use apk::Apk;
mod apt;
pub use apt::Apt;
mod pacman;
pub use pacman::Pacman;
mod yay;
use tracing::info;
pub use yay::Yay;
mod paru;
pub use paru::Paru;

into_boxed_am!(Apk, Apt, Pacman, Yay, Paru);

async fn install(
    u: &User,
    int: &DynInteractor,
    query_args: impl AsRef<str>,
    query_s: &str,
    pm: &str,
    args: &[&str],
) -> Result<bool> {
    use std::iter::once;
    let input = once(query_args.as_ref()).chain(once(query_s));
    let cmd = Script::sh(Box::new(input));
    let pkgs = u.exec(cmd).output().await?;
    if pkgs.is_empty() {
        return Ok(false);
    }
    info!("pkgs[{}] need to be installed", pkgs);
    let args = args.iter().copied();
    let pkgs = pkgs.split_whitespace();
    let s = Script::Split {
        program: pm,
        args: Box::new(args.chain(pkgs)),
    };
    let pp = u.pty(s, WindowSize::default()).await?; //TODO:detect
    //size
    let ec = int.ask(pp).await?;
    if ec != 0 {
        whatever!("unexpected exit status {}", ec);
    }
    Ok(true)
}
