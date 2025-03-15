use std::{collections::HashMap, future::IntoFuture, path::Path};

use dv_api::{
    LocalConfig, Os, Package, SSHConfig, User, UserCast,
    fs::{CheckInfo, Metadata, OpenFlags},
    process::Interactor,
};
use rune::{
    Any,
    runtime::{self, Mut, Object, Ref},
    support,
};
use tracing::info;

use crate::{
    cache::SqliteCache,
    interactor::TermInteractor,
    multi::{Context, action},
};
use support::Result as LRes;

#[derive(Debug, Default)]
struct Device {
    system: Option<String>,
    users: Vec<String>,
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
    #[rune::function(path = Self::user_params)]
    async fn user_params(this: Ref<Self>, id: Ref<str>) -> LRes<Object> {
        let user = this.context().get_user(id).await?;
        let mut obj = Object::new();
        obj.insert(
            rune::alloc::String::try_from("os")?,
            rune::to_value(user.params.os)?,
        )?;
        obj.insert(
            rune::alloc::String::try_from("hid")?,
            rune::to_value(user.hid.as_str())?,
        )?;
        Ok(obj)
    }
    #[rune::function(path = Self::copy)]
    async fn copy(
        this: Ref<Self>,
        src_uid: Ref<str>,
        src_path: Ref<str>,
        dst_uid: Ref<str>,
        dst_path: Ref<str>,
    ) -> LRes<bool> {
        crate::multi::copy(
            &this.context(),
            src_uid,
            src_path,
            dst_uid,
            dst_path.as_ref(),
        )
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
                this.cache.set(id, key, 0).await?;
            }
            res
        };
        action!(this, res, "once {} {}", id, key);
        Ok(res)
    }
    #[rune::function(path = Self::sync)]
    async fn sync(
        this: Ref<Self>,
        src_uid: Ref<str>,
        src_path: Ref<str>,
        dst_uid: Ref<str>,
        dst_path: Ref<str>,
    ) -> LRes<bool> {
        crate::multi::sync(
            &this.context(),
            src_uid,
            src_path,
            dst_uid,
            dst_path.as_ref(),
        )
        .await
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
            let res = user.check_path(path).await?;
            let mut srcs = runtime::Vec::new();
            let copy = async |src: &str| -> LRes<runtime::Value> {
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
                    let len = buf.len();
                    for Metadata { path, .. } in di.files {
                        buf.truncate(len);
                        buf.push_str(&path);
                        srcs.push(copy(&buf).await?)?;
                    }
                }
            }
            Ok(srcs)
        } else {
            Err(rune::support::Error::msg("missing user"))
        }
    }
}

#[rune::function(free,path = LocalConfig::new)]
fn local_config_new() -> LocalConfig {
    LocalConfig {
        hid: "local".to_string(),
        mount: "~/.local/share/dv".into(),
    }
}

impl Dv {
    #[rune::function(path = Self::add_current)]
    async fn add_current(mut this: Mut<Self>, id: Ref<str>, user: LocalConfig) -> LRes<()> {
        let id = id.as_ref();
        let u = user.cast().await.unwrap();
        if this.users.insert(id.to_string(), u).is_some() {
            panic!("user already exists");
        }
        let d = this.devices.entry(id.to_string()).or_default();
        d.users.push(id.to_string());
        let u = &this.users[id];
        this.interactor
            .log(&format!(
                "local user: {:<10}, hid: {:<10}, os: {:<8}",
                id,
                u.hid,
                u.params.os.as_ref()
            ))
            .await;
        Ok(())
    }
}

#[rune::function(free,path = SSHConfig::new)]
fn ssh_config_new(id: &str) -> SSHConfig {
    SSHConfig {
        hid: "local".to_string(),
        mount: "~/.local/share/dv".into(),
        host: id.to_string(),
        is_system: false,
        os: Os::Linux(dv_api::LinuxOs::Unknown),
        passwd: None,
    }
}

impl Dv {
    #[rune::function(path = Self::add_ssh_user)]
    async fn add_ssh_user(mut this: Mut<Self>, id: Ref<str>, user: SSHConfig) -> LRes<()> {
        let id = id.as_ref();
        info!("ssh user: {:?}", user);
        let u = user.cast().await.unwrap();
        let is_system = u.is_system;
        if this.users.insert(id.to_owned(), u).is_some() {
            panic!("user already exists");
        }
        let d = this.devices.entry(id.to_owned()).or_default();
        match (is_system, &mut d.system) {
            (true, Some(_)) => panic!("system user already exists"),
            (true, None) => d.system = Some(id.to_owned()),
            (false, _) => d.users.push(id.to_owned()),
        }
        let u = &this.users[id];
        this.interactor
            .log(&format!(
                "ssh   user: {:<10}, hid: {:<10}, os: {:<8}",
                id,
                u.hid,
                u.params.os.as_ref()
            ))
            .await;
        Ok(())
    }
}

impl Dv {
    #[rune::function(path = Self::app)]
    async fn app(this: Ref<Self>, uid: Ref<str>, packages: Package) -> LRes<bool> {
        let ctx = this.context();
        crate::multi::app(&ctx, uid, packages).await
    }
}
pub fn module() -> Result<rune::Module, rune::ContextError> {
    let mut m = rune::Module::default();
    m.ty::<Dv>()?;
    m.ty::<LocalConfig>()?;
    m.function_meta(local_config_new)?;
    m.function_meta(Dv::add_current)?;
    m.ty::<SSHConfig>()?;
    m.function_meta(ssh_config_new)?;
    m.function_meta(Dv::add_ssh_user)?;
    m.function_meta(Dv::user_params)?;
    m.function_meta(Dv::copy)?;
    crate::multi::register(&mut m)?;
    m.function_meta(Dv::app)?;
    m.function_meta(Dv::auto)?;
    m.function_meta(Dv::exec)?;
    m.function_meta(Dv::once)?;
    m.function_meta(Dv::sync)?;
    m.function_meta(Dv::refresh)?;
    m.function_meta(Dv::load_src)?;
    Ok(m)
}
