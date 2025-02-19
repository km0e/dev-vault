use dv_api::process::{Interactor, Script};

use super::Context;

pub async fn exec(
    ctx: &Context<'_>,
    uid: impl AsRef<str>,
    shell: Option<&str>,
    commands: impl AsRef<str>,
) -> Option<bool> {
    let uid = uid.as_ref();
    let commands = commands.as_ref();
    let script = shell
        .map(|sh| Script::Script {
            program: sh,
            input: Box::new([commands].into_iter()),
        })
        .unwrap_or_else(|| Script::Whole(commands));
    let user = ctx.try_get_user(uid).await?;
    if ctx.dry_run {
        ctx.interactor.log(&format!("[n] exec {}", commands)).await;
        return Some(true);
    }
    let mut pp = ctx.async_assert_result(user.exec(script)).await?;

    let ec = ctx.async_assert_result(ctx.interactor.ask(&mut pp)).await?;
    ctx.assert_bool(ec == 0, || format!("unexpect exit code: {}", ec))
        .await?;
    Some(true)
}
