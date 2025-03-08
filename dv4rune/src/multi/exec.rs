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
        .map(|sh| Script::Script {
            program: sh,
            input: Box::new([commands].into_iter()),
        })
        .unwrap_or_else(|| Script::Whole(commands));
    let user = ctx.get_user(uid).await?;
    if !ctx.dry_run {
        let pp = user.exec(script).log(ctx.interactor).await?;

        let ec = ctx.interactor.ask(pp).log(ctx.interactor).await?;
        if ec != 0 {
            let msg = format!("unexpect exit code: {}", ec);
            ctx.interactor.log(&msg).await;
            Err(rune::support::Error::msg(msg))?
        }
    }
    action!(ctx, true, "exec {}", commands);
    Ok(true)
}
