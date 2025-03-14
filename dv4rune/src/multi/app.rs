use dv_api::Package;

use super::dev::*;

#[rune::function(instance, protocol=ADD_ASSIGN)]
fn merge(this: &mut Package, value: &Package) {
    let insert_or_merge = |mut l: &mut Option<String>, r: &Option<String>| match (&mut l, r) {
        (None, Some(v)) => *l = Some(v.clone()),
        (Some(l), Some(r)) => {
            l.push(' ');
            l.push_str(r)
        }
        _ => (),
    };
    insert_or_merge(&mut this.apk, &value.apk);
    insert_or_merge(&mut this.apt, &value.apt);
    insert_or_merge(&mut this.pacman, &value.pacman);
    insert_or_merge(&mut this.yay, &value.yay);
    insert_or_merge(&mut this.paru, &value.paru);
    insert_or_merge(&mut this.winget, &value.winget);
}

#[rune::function(free, path = Package::new)]
fn new() -> Package {
    Package::default()
}

#[rune::function(instance, path = set)]
fn set(this: &mut Package, key: &str, value: &str) {
    match key {
        "apk" => this.apk = Some(value.to_string()),
        "apt" => this.apt = Some(value.to_string()),
        "pacman" => this.pacman = Some(value.to_string()),
        "yay" => this.yay = Some(value.to_string()),
        "paru" => this.paru = Some(value.to_string()),
        "winget" => this.winget = Some(value.to_string()),
        _ => (),
    }
}

pub async fn app(ctx: &Context<'_>, uid: impl AsRef<str>, packages: Package) -> LRes<bool> {
    let uid = uid.as_ref();
    let user = ctx.get_user(uid).await?;
    let res = ctx.dry_run
        || user
            .app(ctx.interactor, &packages)
            .log(ctx.interactor)
            .await?;
    action!(ctx, res, "app {}", &packages);
    Ok(res)
}

pub fn register(m: &mut rune::module::Module) -> Result<(), rune::ContextError> {
    m.ty::<Package>()?;
    m.function_meta(new)?;
    m.function_meta(merge)?;
    m.function_meta(set)?;
    Ok(())
}
