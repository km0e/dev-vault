use super::dev::*;

pub async fn auto(
    ctx: &Context<'_>,
    uid: impl AsRef<str>,
    service: impl AsRef<str>,
    action: impl AsRef<str>,
    args: Option<&str>,
) -> LRes<bool> {
    let uid = uid.as_ref();
    let service = service.as_ref();
    let action = action.as_ref();
    let user = ctx.get_user(uid).await?;
    if !ctx.dry_run {
        user.auto(service, action, args).log(ctx.interactor).await?;
    };
    action!(ctx, true, "auto {} {}", service, action);
    Ok(true)
}
