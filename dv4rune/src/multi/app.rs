use super::dev::*;

pub async fn app(ctx: &Context<'_>, uid: impl AsRef<str>, packages: String) -> LRes<bool> {
    let uid = uid.as_ref();
    let user = ctx.get_user(uid).await?;
    let res = ctx.dry_run
        || user
            .app(ctx.interactor, &packages)
            .log(ctx.interactor)
            .await?;
    action!(ctx, res, "app {}", packages);
    Ok(res)
}
