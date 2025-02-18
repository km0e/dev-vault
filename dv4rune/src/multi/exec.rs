use dv_api::process::{Interactor, Script};

use crate::{
    dvl::Context,
    utils::{assert_bool, assert_option, assert_result},
};

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
    let user = assert_option!(ctx.users.get(uid), ctx.interactor, || format!(
        "user {} not found",
        uid
    ));
    if ctx.dry_run {
        ctx.interactor.log(&format!("[n] exec {}", commands)).await;
        return Some(true);
    }
    let mut pp = assert_result!(user.exec(script).await, ctx.interactor);

    let ec = assert_result!(ctx.interactor.ask(&mut pp).await, ctx.interactor);
    assert_bool!(ec == 0, ctx.interactor, || "exec failed");
    Some(true)
}
