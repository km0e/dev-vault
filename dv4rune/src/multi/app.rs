use dv_api::process::Interactor;

use crate::utils::LogFutResult;

use super::{Context, LRes};

pub async fn app(ctx: &Context<'_>, uid: impl AsRef<str>, packages: String) -> LRes<bool> {
    let uid = uid.as_ref();
    let user = ctx.get_user(uid).await?;
    if ctx.dry_run {
        ctx.interactor.log(&format!("[n] app {}", packages)).await;
        return Ok(true);
    }
    let res = user
        .app(ctx.interactor, &packages)
        .log(ctx.interactor)
        .await?;
    Ok(res)
}
