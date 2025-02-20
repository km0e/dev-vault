use dv_api::process::Interactor;

use crate::utils::LogFutResult;

use super::{Context, LRes};

pub async fn auto(
    ctx: &Context<'_>,
    uid: impl AsRef<str>,
    service: impl AsRef<str>,
    action: impl AsRef<str>,
) -> LRes<bool> {
    let uid = uid.as_ref();
    let service = service.as_ref();
    let action = action.as_ref();
    let user = ctx.get_user(uid).await?;
    if ctx.dry_run {
        ctx.interactor
            .log(&format!("[n] auto {} {}", service, action))
            .await;
        return Ok(true);
    }
    user.auto(service, action).log(ctx.interactor).await?;
    Ok(true)
}
