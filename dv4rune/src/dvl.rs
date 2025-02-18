use std::{collections::HashMap, path::Path};

use dv_api::{process::Interactor, User, UserCast};
use rune::{
    runtime::{self, Mut, Ref},
    Any,
};
use tracing::info;

use crate::{
    cache::SqliteCache,
    interactor::TermInteractor,
    utils::{assert_option, assert_result, field, obj2, obj_take2, value2},
};

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
}

pub struct Context<'a> {
    pub dry_run: bool,
    pub cache: &'a SqliteCache,
    pub interactor: &'a TermInteractor,
    pub users: &'a HashMap<String, User>,
}

impl Context<'_> {
    pub async fn get_user(&self, uid: impl AsRef<str>) -> Option<&User> {
        Some(assert_option!(
            self.users.get(uid.as_ref()),
            &self.interactor,
            || format!("user {} not found", uid.as_ref())
        ))
    }
}

impl Dv {
    #[rune::function(path = Self::add_local)]
    async fn add_current(
        mut this: Mut<Self>,
        id: Ref<str>,
        mut user: Mut<runtime::Object>,
    ) -> Option<()> {
        use dv_api::LocalConfig;
        let user =
            obj2!(LocalConfig, &this.interactor, user, hid@("local"), mount@("~/.config/dv"));
        let u = user.cast().await.unwrap();
        if this.users.insert(id.to_owned(), u).is_some() {
            panic!("user already exists");
        }
        let d = this.devices.entry(id.to_owned()).or_default();
        d.users.push(id.to_owned());
        this.interactor
            .log(&format!("add local user: {}", id.as_ref()))
            .await;
        Some(())
    }
    #[rune::function(path = Self::add_ssh_user)]
    async fn add_ssh_user(
        mut this: Mut<Self>,
        id: Ref<str>,
        mut user: Mut<runtime::Object>,
    ) -> Option<()> {
        use dv_api::SSHConfig;
        let id = id.as_ref();
        let user = obj2!(SSHConfig, &this.interactor, user, hid@(id), host@(id), is_system@(,bool),os@("linux"), @default);
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
        this.interactor.log(&format!("add ssh user: {}", id)).await;
        Some(())
    }
    fn context(&self) -> Context<'_> {
        Context {
            dry_run: self.dry_run,
            cache: &self.cache,
            interactor: &self.interactor,
            users: &self.users,
        }
    }
    #[rune::function(path = Self::copy)]
    async fn copy(
        this: Ref<Self>,
        src_uid: Ref<str>,
        src_path: Ref<str>,
        dst_uid: Ref<str>,
        dst_path: Ref<str>,
    ) -> Option<bool> {
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
    ) -> Option<bool> {
        crate::multi::exec(&this.context(), uid, shell.as_deref(), commands).await
    }
    #[rune::function(path = Self::app)]
    async fn app(this: Ref<Self>, uid: Ref<str>, package: runtime::Vec) -> Option<bool> {
        let ctx = this.context();
        let res: Result<String, String> =
            package.into_iter().try_fold(String::new(), |mut acc, n| {
                if !acc.is_empty() {
                    acc.push(' ');
                }
                acc.push_str(&value2!(n)?);
                Ok(acc)
            });
        let packages = assert_result!(res, ctx.interactor);
        crate::multi::app(ctx, uid, packages).await
    }
    #[rune::function(path = Self::auto)]
    async fn auto(
        this: Ref<Self>,
        uid: Ref<str>,
        service: Ref<str>,
        action: Ref<str>,
    ) -> Option<bool> {
        crate::multi::auto(&this.context(), uid, service, action).await
    }
}
