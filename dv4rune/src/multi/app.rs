use dv_api::process::Interactor;

use super::Context;

pub async fn app(ctx: &Context<'_>, uid: impl AsRef<str>, packages: String) -> Option<bool> {
    let uid = uid.as_ref();
    let user = ctx.try_get_user(uid).await?;
    if ctx.dry_run {
        ctx.interactor.log(&format!("[n] app {}", packages)).await;
        return Some(true);
    }
    ctx.async_assert_result(user.app(ctx.interactor, &packages))
        .await
}
