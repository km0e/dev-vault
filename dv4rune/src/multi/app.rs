use dv_api::process::Interactor;

use crate::{
    dvl::Context,
    utils::{assert_option, assert_result},
};

pub async fn app(ctx: Context<'_>, uid: impl AsRef<str>, packages: String) -> Option<bool> {
    let uid = uid.as_ref();
    let user = assert_option!(ctx.users.get(uid), ctx.interactor, || format!(
        "user {} not found",
        uid
    ));
    if ctx.dry_run {
        ctx.interactor.log(&format!("[n] app {}", packages)).await;
        return Some(true);
    }
    let res = assert_result!(user.app(ctx.interactor, &packages).await, ctx.interactor);
    Some(res)
}
