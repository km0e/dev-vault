use dv_api::process::{Interactor, Script};

use crate::utils::LogFutResult;

use super::{Context, LRes};

pub async fn exec(
    ctx: &Context<'_>,
    uid: impl AsRef<str>,
    shell: Option<&str>,
    commands: impl AsRef<str>,
) -> LRes<bool> {
    let uid = uid.as_ref();
    let commands = commands.as_ref();
    let script = shell
        .map(|sh| Script::Script {
            program: sh,
            input: Box::new([commands].into_iter()),
        })
        .unwrap_or_else(|| Script::Whole(commands));
    let user = ctx.get_user(uid).await?;
    if ctx.dry_run {
        ctx.interactor.log(&format!("[n] exec {}", commands)).await;
        return Ok(true);
    }
    let mut pp = user.exec(script).log(ctx.interactor).await?;

    let ec = ctx.interactor.ask(&mut pp).log(ctx.interactor).await?;
    if ec != 0 {
        ctx.interactor
            .log(&format!("unexpect exit code: {}", ec))
            .await;
    }
    Ok(true)
}
