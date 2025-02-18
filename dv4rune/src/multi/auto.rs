use dv_api::process::Interactor;

use crate::{
    dvl::Context,
    utils::{assert_option, assert_result},
};

pub async fn auto(
    ctx: &Context<'_>,
    uid: impl AsRef<str>,
    service: impl AsRef<str>,
    action: impl AsRef<str>,
) -> Option<bool> {
    let uid = uid.as_ref();
    let service = service.as_ref();
    let action = action.as_ref();
    let user = assert_option!(ctx.users.get(uid), ctx.interactor, || format!(
        "user {} not found",
        uid
    ));
    if ctx.dry_run {
        ctx.interactor
            .log(&format!("[n] auto {} {}", service, action))
            .await;
        return Some(true);
    }
    assert_result!(user.auto(service, action).await, ctx.interactor);
    Some(true)
}
