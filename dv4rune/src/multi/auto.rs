use dv_api::process::Interactor;

use super::Context;

pub async fn auto(
    ctx: &Context<'_>,
    uid: impl AsRef<str>,
    service: impl AsRef<str>,
    action: impl AsRef<str>,
) -> Option<bool> {
    let uid = uid.as_ref();
    let service = service.as_ref();
    let action = action.as_ref();
    let user = ctx.try_get_user(uid).await?;
    if ctx.dry_run {
        ctx.interactor
            .log(&format!("[n] auto {} {}", service, action))
            .await;
        return Some(true);
    }
    ctx.async_assert_result(user.auto(service, action)).await?;
    Some(true)
}
