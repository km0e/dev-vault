use std::collections::HashMap;

use dv_api::Pm;

use super::dev::*;

#[derive(Debug, Default, rune::Any)]
pub struct Package {
    pm: HashMap<Pm, String>,
}

impl std::fmt::Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.pm.is_empty() {
            write!(f, "empty")
        } else {
            for (pm, package) in &self.pm {
                write!(f, "{}:{} ", pm, package)?;
            }
            Ok(())
        }
    }
}

impl Package {
    pub fn as_package(&self) -> dv_api::Package {
        dv_api::Package {
            pm: self.pm.iter().map(|(k, v)| (*k, v.as_str())).collect(),
        }
    }

    #[rune::function(path = Self::new)]
    pub fn new() -> Package {
        Package::default()
    }

    #[rune::function(path = Self::add_assign, protocol=ADD_ASSIGN)]
    fn add_assign(this: &mut Package, value: &Package) {
        for (k, v) in &value.pm {
            let entry = this.pm.entry(*k).or_default();
            entry.push(' ');
            entry.push_str(v);
        }
    }

    #[rune::function(path = Self::index_set, protocol = INDEX_SET)]
    pub fn index_set(&mut self, key: &str, value: &str) -> LRes<()> {
        self.pm.insert(key.parse()?, value.to_string());
        Ok(())
    }
}

pub async fn pm(ctx: &Context<'_>, uid: impl AsRef<str>, packages: Package) -> LRes<bool> {
    let uid = uid.as_ref();
    let user = ctx.get_user(uid).await?;
    let res = ctx.dry_run
        || user
            .app(ctx.interactor, packages.as_package())
            .log(ctx.interactor)
            .await?;
    action!(ctx, res, "app {}", packages);
    Ok(res)
}

pub fn register(m: &mut rune::module::Module) -> Result<(), rune::ContextError> {
    m.ty::<Package>()?;
    m.function_meta(Package::new)?;
    m.function_meta(Package::add_assign)?;
    m.function_meta(Package::index_set)?;
    Ok(())
}
