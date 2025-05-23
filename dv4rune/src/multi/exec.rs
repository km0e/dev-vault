use super::dev::*;
use dv_api::process::Script;

pub async fn exec(
    ctx: &Context<'_>,
    uid: impl AsRef<str>,
    shell: Option<&str>,
    commands: impl AsRef<str>,
) -> LRes<bool> {
    let uid = uid.as_ref();
    let commands = commands.as_ref();
    let script = shell
        .map(|_sh| Script::sh(Box::new([commands].into_iter())))
        .unwrap_or_else(|| Script::Whole(commands));
    let user = ctx.get_user(uid)?;
    if !ctx.dry_run {
        let pp = user
            .pty(script, ctx.interactor.window_size().await)
            .log(ctx.interactor)
            .await?;

        let ec = ctx.interactor.ask(pp).log(ctx.interactor).await?;
        if ec != 0 {
            let msg = format!("unexpect exit code: {}", ec);
            ctx.interactor.log(msg.clone()).await;
            Err(rune::support::Error::msg(msg))?
        }
    }
    action!(ctx, true, "exec {}", commands);
    Ok(true)
}
