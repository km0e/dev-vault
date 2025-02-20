use std::{collections::HashMap, future::IntoFuture, path::Path};

use dv_api::{process::Interactor, Os, User, UserCast};
use rune::{
    runtime::{self, Mut, Object, Ref, VmError},
    support, Any,
};
use tracing::info;

use crate::{
    cache::SqliteCache,
    interactor::TermInteractor,
    multi::Context,
    utils::{field, obj2, obj_take2, value2, LogResult},
};
use support::Result as LRes;

#[derive(Debug, Default)]
struct Device {
    system: Option<String>,
    users: Vec<String>,
}

#[derive(Debug, Any)]
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
            interactor: Default::default(),
        }
    }
    fn context(&self) -> Context<'_> {
        Context::new(self.dry_run, &self.cache, &self.interactor, &self.users)
    }
}
impl Dv {
    #[rune::function(path = Self::add_local)]
    async fn add_current(mut this: Mut<Self>, id: Ref<str>, mut user: Mut<Object>) -> LRes<()> {
        let id = id.as_ref();
        use dv_api::LocalConfig;
        let user =
            obj2!(LocalConfig, &this.context(), user, hid@("local",), mount@("~/.config/dv",));
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
    #[rune::function(path = Self::add_ssh_user)]
    async fn add_ssh_user(mut this: Mut<Self>, id: Ref<str>, mut user: Mut<Object>) -> LRes<()> {
        use dv_api::SSHConfig;
        let id = id.as_ref();
        let user = obj2!(SSHConfig, &this.context(), user, hid@(id,), host@(id,), is_system@(bool),os@("linux", Os::from), @default);
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
    #[rune::function(path = Self::user_params)]
    async fn user_params(this: Ref<Self>, id: Ref<str>) -> LRes<Object> {
        let user = this.context().get_user(id).await?;
        let mut obj = Object::new();
        obj.insert(
            rune::alloc::String::try_from("os")?,
            rune::to_value(user.params.os.as_ref())?,
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
    #[rune::function(path = Self::app)]
    async fn app(this: Ref<Self>, uid: Ref<str>, package: runtime::Vec) -> LRes<bool> {
        let ctx = this.context();
        let res: Result<String, VmError> =
            package.into_iter().try_fold(String::new(), |mut acc, n| {
                if !acc.is_empty() {
                    acc.push(' ');
                }
                acc.push_str(&value2!(n)?);
                Ok(acc)
            });

        crate::multi::app(&ctx, uid, res.log(ctx.interactor).await?).await
    }
    #[rune::function(path = Self::auto)]
    async fn auto(
        this: Ref<Self>,
        uid: Ref<str>,
        service: Ref<str>,
        action: Ref<str>,
    ) -> LRes<bool> {
        crate::multi::auto(&this.context(), uid, service, action).await
    }
    #[rune::function(path = Self::once)]
    async fn once(this: Ref<Self>, id: Ref<str>, f: runtime::Function) -> LRes<bool> {
        let id = id.as_ref();
        let b = this.cache.get(id, "").await?;
        if b.is_some() {
            this.interactor.log(&format!("skip {}", id)).await;
            return Ok(false);
        }
        let res: LRes<bool> = rune::from_value(
            f.call::<_, runtime::Future>(())
                .into_result()?
                .into_future()
                .await
                .into_result()?,
        )?;
        let res = res?;
        if res && !this.dry_run {
            this.interactor.log(&format!("exec {}", id)).await;
            this.cache.set(id, "", 0).await?;
        }
        Ok(res)
    }
}

pub fn module() -> Result<rune::Module, rune::ContextError> {
    let mut m = rune::Module::default();
    m.ty::<Dv>()?;
    m.function_meta(Dv::add_current)?;
    m.function_meta(Dv::add_ssh_user)?;
    m.function_meta(Dv::user_params)?;
    m.function_meta(Dv::copy)?;
    m.function_meta(Dv::app)?;
    m.function_meta(Dv::auto)?;
    m.function_meta(Dv::exec)?;
    m.function_meta(Dv::once)?;
    Ok(m)
}
