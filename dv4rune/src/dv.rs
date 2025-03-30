use std::{collections::HashMap, future::IntoFuture, path::Path, sync::Arc};

use dv_api::{
    Config, Os, User,
    dev::Dev,
    fs::{CheckInfo, Metadata, OpenFlags},
    process::Interactor,
    user::Utf8Path,
    whatever,
};
use resplus::attach;
use rune::{
    Any,
    runtime::{self, Mut, Ref},
    support,
};
use tracing::info;

use crate::{
    cache::SqliteCache,
    interactor::TermInteractor,
    multi::{Context, Package, action},
};
use support::Result as LRes;

#[derive(Debug)]
struct Device {
    dev: Arc<Dev>,
    system: Option<String>,
    users: Vec<String>,
}

impl Device {
    fn new(dev: Arc<Dev>) -> Self {
        Self {
            dev,
            system: None,
            users: Vec::new(),
        }
    }
}

#[derive(Any)]
pub struct Dv {
    dry_run: bool,
    devices: HashMap<String, Device>,
    users: HashMap<String, User>,
    cache: SqliteCache,
    interactor: TermInteractor,
}

impl Dv {
    pub fn new(path: impl AsRef<Path>, dry_run: bool) -> Self {
        Self {
            dry_run,
            devices: HashMap::new(),
            users: HashMap::new(),
            cache: SqliteCache::new(path),
            interactor: TermInteractor::new().unwrap(),
        }
    }
    fn context(&self) -> Context<'_> {
        Context::new(self.dry_run, &self.cache, &self.interactor, &self.users)
    }
}
impl Dv {
    #[rune::function(path = Self::copy)]
    async fn copy(
        this: Ref<Self>,
        src: (Ref<str>, Ref<str>),
        dst: (Ref<str>, Ref<str>),
        confirm: Option<Ref<str>>,
    ) -> LRes<bool> {
        crate::multi::CopyContext::new(this.context(), &src.0, &dst.0, confirm.as_deref())
            .await?
            .copy(src.1, dst.1)
            .await
    }
    #[rune::function(path = Self::exec)]
    async fn exec(
        this: Ref<Self>,
        uid: Ref<str>,
        shell: Option<Ref<str>>,
        commands: Ref<str>,
    ) -> LRes<bool> {
        crate::multi::exec(&this.context(), uid, shell.as_deref(), commands).await
    }
    #[rune::function(path = Self::auto)]
    async fn auto(
        this: Ref<Self>,
        uid: Ref<str>,
        service: Ref<str>,
        action: Ref<str>,
        args: Option<Ref<str>>,
    ) -> LRes<bool> {
        let args = args.as_ref().map(|s| s.as_ref());
        crate::multi::auto(&this.context(), uid, service, action, args).await
    }
    #[rune::function(path = Self::once)]
    async fn once(
        this: Ref<Self>,
        id: Ref<str>,
        key: Ref<str>,
        f: runtime::Function,
    ) -> LRes<bool> {
        let id = id.as_ref();
        let key = key.as_ref();
        let b = this.cache.get(id, key).await?;
        let res = b.is_none()//not cached
        && {
            info!("once {} {}", id,key);
            let res: LRes<bool> = rune::from_value(
                f.call::<runtime::Future>(())
                    .into_result()?
                    .into_future()
                    .await
                    .into_result()?,
            )?;
            let res= res?;
            if !this.dry_run {
                this.cache.set(id, key, 0,0).await?;
            }
            res
        };
        action!(this, res, "once {} {}", id, key);
        Ok(res)
    }
    #[rune::function(path = Self::refresh)]
    async fn refresh(this: Ref<Self>, id: Ref<str>, key: Ref<str>) -> LRes<bool> {
        this.cache.del(&id, &key).await?;
        action!(this, true, "refresh {} {}", id.as_ref(), key.as_ref());
        Ok(true)
    }
    #[rune::function(path = Self::load_src)]
    async fn load_src(this: Ref<Self>, id: Ref<str>, path: Ref<str>) -> LRes<runtime::Vec> {
        let id = id.as_ref();
        if let Some(user) = this.users.get(id) {
            let path = path.as_ref();
            let res = attach!(user.check_path(path), ..).await?;
            let mut srcs = runtime::Vec::new();
            let copy = async |src: &Utf8Path| -> LRes<runtime::Value> {
                let mut src = user.open(src, OpenFlags::READ).await?;
                let dst = tempfile::NamedTempFile::new()?;
                let (file, path) = dst.keep()?;
                let mut file = tokio::fs::File::from_std(file);
                tokio::io::copy(&mut src, &mut file).await?;
                Ok(rune::to_value(path.to_string_lossy().to_string())?)
            };
            match res {
                CheckInfo::File(f) => {
                    srcs.push(copy(&f.path).await?)?;
                }
                CheckInfo::Dir(di) => {
                    let mut buf = di.path;
                    for Metadata { path, .. } in di.files {
                        buf.push(&path);
                        srcs.push(copy(&buf).await?)?;
                        buf.pop();
                    }
                }
            }
            Ok(srcs)
        } else {
            Err(rune::support::Error::msg("missing user"))
        }
    }
}

impl Dv {
    #[rune::function(path = Self::add_user)]
    async fn add_user(mut this: Mut<Dv>, id: Ref<str>, cfg: Config) -> LRes<()> {
        let id = id.as_ref();
        if this.users.contains_key(id) {
            whatever!("user {} already exists", id);
        }
        let u = if let Some(hid) = cfg.hid() {
            let hid = hid.to_string();
            if let Some(dev) = this.devices.get_mut(&hid) {
                let u = cfg.connect(Some(dev.dev.clone())).await?;
                if u.is_system {
                    dev.system = Some(id.to_string());
                } else {
                    dev.users.push(id.to_string());
                }
                u
            } else {
                let u = cfg.connect(None).await?;
                let mut dev = Device::new(u.dev.clone());
                if u.is_system {
                    dev.system = Some(id.to_string());
                } else {
                    dev.users.push(id.to_string());
                }
                this.devices.insert(hid.to_string(), dev);
                u
            }
        } else {
            cfg.connect(None).await?
        };
        this.interactor
            .log(format!("user: {:<10}, os: {:<8}", id, u.dev.os.as_ref()))
            .await;
        this.users.insert(id.to_string(), u);
        Ok(())
    }
}

impl Dv {
    #[rune::function(path = Self::pm)]
    async fn pm(this: Ref<Self>, uid: Ref<str>, packages: Package) -> LRes<bool> {
        crate::multi::pm(this.context(), uid.as_ref(), packages).await
    }
}

impl Dv {
    #[rune::function(path = Self::os)]
    async fn os(this: Ref<Self>, uid: Ref<str>) -> LRes<Os> {
        let uid = uid.as_ref();
        let user = this.context().get_user(uid).await?;
        Ok(user.dev.os)
    }
}

pub fn module() -> Result<rune::Module, rune::ContextError> {
    let mut m = rune::Module::default();
    m.ty::<Dv>()?;
    crate::multi::register(&mut m)?;
    m.function_meta(Dv::add_user)?;
    m.function_meta(Dv::copy)?;
    m.function_meta(Dv::pm)?;
    m.function_meta(Dv::auto)?;
    m.function_meta(Dv::exec)?;
    m.function_meta(Dv::once)?;
    m.function_meta(Dv::refresh)?;
    m.function_meta(Dv::load_src)?;
    m.function_meta(Dv::os)?;
    Ok(m)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use dv_api::User;
    use std::collections::HashMap;

    pub struct TestDv {
        pub dry_run: bool,
        pub users: HashMap<String, User>,
        pub cache: SqliteCache,
        pub interactor: TermInteractor,
    }
    impl TestDv {
        pub fn context(&self) -> Context<'_> {
            Context::new(self.dry_run, &self.cache, &self.interactor, &self.users)
        }
    }
}
