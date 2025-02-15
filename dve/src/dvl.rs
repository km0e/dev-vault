use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
};

use dev_vault::user::{Metadata, OpenFlags, Script, User, UserCast};
use rune::{
    runtime::{self, Mut, Ref},
    Any,
};
use tracing::debug;

use crate::{cache::SqliteCache, interactor::TermInteractor};

#[derive(Debug, Default)]
struct Device {
    system: Option<String>,
    users: Vec<String>,
}

#[derive(Debug, Any)]
pub struct Dv {
    dry_run: bool,
    devices: HashMap<String, Device>,
    users: HashMap<String, dev_vault::user::User>,
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

#[derive(Any, Clone)]
pub struct CurrentUser {
    hid: String,
    mount: PathBuf,
}

impl CurrentUser {
    #[rune::function(path = Self::new)]
    fn new(hid: Ref<str>, mount: Ref<str>) -> Self {
        Self {
            hid: hid.as_ref().into(),
            mount: mount.as_ref().into(),
        }
    }
    fn cast(self) -> dev_vault::user::LocalConfig {
        dev_vault::user::LocalConfig {
            hid: self.hid,
            mount: self.mount,
        }
    }
}

#[derive(Any, Clone)]
pub struct SSHUser {
    hid: String,
    host: String,
    #[rune(get, set)]
    is_system: bool,
    os: Option<String>,
    passwd: Option<String>,
}

impl SSHUser {
    #[rune::function(path = Self::new)]
    fn new(hid: Ref<str>, host: Ref<str>) -> Self {
        Self {
            hid: hid.as_ref().into(),
            host: host.as_ref().into(),
            is_system: false,
            os: None,
            passwd: None,
        }
    }
    fn cast(self) -> dev_vault::user::SSHUserConfig {
        let mut ssh_user = dev_vault::user::SSHUserConfig::new(self.hid, self.host);
        ssh_user.is_system = self.is_system;
        ssh_user.os = self.os;
        ssh_user.passwd = self.passwd;
        ssh_user
    }
}
macro_rules! assert_option {
    ($o:expr, $interactor:expr, $m:expr) => {
        match $o {
            Some(v) => v,
            None => {
                fn _f<S: Into<String>>(f: impl FnOnce() -> S) -> String {
                    f().into()
                }
                let _s = _f($m);
                $interactor.log(&_s).await;
                return Err(_s);
            }
        }
    };
}

macro_rules! assert_bool {
    ($b:expr, $interactor:expr, $m:expr) => {
        assert_option!(($b).then_some(()), $interactor, $m)
    };
}

macro_rules! assert_result {
    ($r:expr, $interactor:expr, $m:expr) => {
        match $r {
            Ok(v) => v,
            Err(e) => {
                fn _f(e: String, f: impl FnOnce(String) -> impl Into<String>) -> String {
                    f(e).into()
                }
                $interactor.log(&_f(e.to_string(), $m)).await;
                return Err(());
            }
        }
    };
    ($r:expr, $interactor:expr) => {
        match $r {
            Ok(v) => v,
            Err(e) => {
                let _s = e.to_string();
                $interactor.log(&_s).await;
                return Err(_s);
            }
        }
    };
}
impl Dv {
    #[rune::function(path = Self::add_local)]
    async fn add_local(mut this: Mut<Self>, id: Ref<str>, user: Ref<CurrentUser>) {
        println!("add_local");
        let u = user.clone().cast().cast().await.unwrap();
        this.interactor
            .log(&format!("add local user: {}", id.as_ref()))
            .await;
        if this.users.insert(id.to_owned(), u).is_some() {
            panic!("user already exists");
        }
        let d = this.devices.entry(id.to_owned()).or_default();
        d.users.push(id.to_owned());
        this.interactor
            .log(&format!("add local user: {}", id.as_ref()))
            .await;
    }
    #[rune::function(path = Self::add_ssh_user)]
    async fn add_ssh_user(mut this: Mut<Self>, id: Ref<str>, user: Ref<SSHUser>) {
        let u = user.clone().cast().cast().await.unwrap();
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
        this.interactor
            .log(&format!("add ssh user: {}", id.as_ref()))
            .await;
    }
    pub async fn check_copy_file(
        &self,
        src_uid: &str,
        src: &User,
        src_path: &str,
        dst_uid: &str,
        dst_path: &str,
        ts: u64,
    ) -> Result<bool, String> {
        let dst = assert_option!(self.users.get(dst_uid), &self.interactor, || format!(
            "dst user {} not found",
            dst_uid
        ));

        let cache = assert_result!(self.cache.get(dst_uid, dst_path).await, &self.interactor);
        let res = if cache.is_some_and(|dst_ts| {
            if dst_ts != ts {
                debug!("{} != {}", dst_ts, ts);
            }
            dst_ts == ts
        }) {
            false
        } else if self.dry_run {
            true
        } else {
            if src.hid != dst.hid {
                let mut src =
                    assert_result!(src.open(src_path, OpenFlags::READ).await, &self.interactor);
                let mut dst = assert_result!(
                    dst.open(dst_path, OpenFlags::WRITE | OpenFlags::CREATE)
                        .await,
                    &self.interactor
                );
                assert_result!(tokio::io::copy(&mut src, &mut dst).await, &self.interactor);
            } else {
                let main = if src.is_system { src } else { dst };
                assert_result!(main.copy(src_path, dst_path).await, &self.interactor);
            }
            true
        };
        if res {
            assert_result!(
                self.cache.set(dst_uid, dst_path, ts).await,
                &self.interactor
            );
        }
        self.interactor
            .log(&format!(
                "[{}] {} copy {}:{} -> {}:{}",
                if self.dry_run { "n" } else { "a" },
                if res { "exec" } else { "skip" },
                src_uid,
                src_path,
                dst_uid,
                dst_path
            ))
            .await;
        Ok(res)
    }
    #[rune::function(path = Self::copy)]
    async fn copy(
        this: Ref<Self>,
        src_uid: Ref<str>,
        src_path: Ref<str>,
        dst_uid: Ref<str>,
        dst_path: Ref<str>,
    ) -> Result<bool, String> {
        assert_bool!(src_path.len() != 0, &this.interactor, || {
            "src_path is empty"
        });
        assert_bool!(dst_path.len() != 0, &this.interactor, || {
            "dst_path is empty"
        });
        let src_uid = src_uid.as_ref();
        let dst_uid = dst_uid.as_ref();
        let src_path = src_path.as_ref();
        let dst_path = dst_path.as_ref();
        let src = assert_option!(this.users.get(src_uid), &this.interactor, || format!(
            "src user {} not found",
            src_uid
        ));
        if src_path.ends_with('/') {
            let meta = assert_result!(src.glob_with_meta(src_path).await, &this.interactor);
            if dst_path.ends_with('/') {
                unimplemented!()
            } else {
                let mut success = false;
                for Metadata { path, ts } in meta {
                    let src_path = format!("{}{}", src_path, path);
                    let dst_path = format!("{}/{}", dst_path, path);
                    let res = assert_result!(
                        this.check_copy_file(src_uid, src, &src_path, dst_uid, &dst_path, ts)
                            .await,
                        &this.interactor
                    );
                    success |= res;
                }
                Ok(success)
            }
        } else {
            let meta = assert_result!(src.check_file(src_path).await, &this.interactor);
            let Metadata { path, ts } = assert_option!(meta.into(), &this.interactor, || format!(
                "src file {} not found",
                src_path
            ));
            let dst_path = dst_path
                .strip_suffix('/')
                .map(|_| {
                    format!(
                        "{}{}",
                        dst_path,
                        src_path
                            .rsplit_once('/')
                            .map(|(_, name)| name)
                            .unwrap_or(src_path)
                    )
                    .into()
                })
                .unwrap_or(Cow::Borrowed(dst_path));
            Ok(assert_result!(
                this.check_copy_file(src_uid, src, &path, dst_uid, &dst_path, ts)
                    .await,
                &this.interactor
            ))
        }
    }
    #[rune::function(path = Self::exec)]
    async fn exec(
        this: Ref<Self>,
        uid: Ref<str>,
        shell: Option<Ref<str>>,
        commands: Ref<str>,
    ) -> Result<(), String> {
        let script = shell
            .as_deref()
            .map(|sh| Script::Script {
                program: sh,
                input: Box::new([commands.as_ref()].into_iter()),
            })
            .unwrap_or_else(|| Script::Whole(commands.as_ref()));
        let user = assert_option!(this.users.get(uid.as_ref()), &this.interactor, || format!(
            "user {} not found",
            uid.as_ref()
        ));
        if this.dry_run {
            this.interactor
                .log(&format!("[n] exec {}", commands.as_ref()))
                .await;
            return Ok(());
        }
        let mut pp = assert_result!(user.exec(script).await, &this.interactor);

        let ec = assert_result!(this.interactor.ask(&mut pp).await, &this.interactor);
        assert_bool!(ec == 0, &this.interactor, || "exec failed");
        Ok(())
    }
    #[rune::function(path = Self::app)]
    async fn app(this: Ref<Self>, uid: Ref<str>, package: runtime::Vec) -> Result<(), String> {
        let uid = uid.as_ref();
        let user = assert_option!(this.users.get(uid), &this.interactor, || format!(
            "user {} not found",
            uid
        ));

        let res: Result<String, String> =
            package.into_iter().try_fold(String::new(), |mut acc, n| {
                if !acc.is_empty() {
                    acc.push(' ');
                }
                acc.push_str(
                    n.into_string()
                        .into_result()
                        .map_err(|e| e.to_string())?
                        .into_ref()
                        .map_err(|e| e.to_string())?
                        .as_ref(),
                );
                Ok(acc)
            });
        let packages = assert_result!(res, &this.interactor);
        if this.dry_run {
            this.interactor.log(&format!("[n] app {}", packages)).await;
            return Ok(());
        }
        let mut pp = assert_result!(user.app(&packages).await, &this.interactor);
        let ec = assert_result!(this.interactor.ask(&mut pp).await, &this.interactor);
        assert_bool!(ec == 0, &this.interactor, || "app failed");
        Ok(())
    }
    #[rune::function(path = Self::auto)]
    async fn auto(
        this: Ref<Self>,
        uid: Ref<str>,
        service: Ref<str>,
        action: Ref<str>,
    ) -> Result<(), String> {
        let uid = uid.as_ref();
        let service = service.as_ref();
        let action = action.as_ref();
        let user = assert_option!(this.users.get(uid), &this.interactor, || format!(
            "user {} not found",
            uid
        ));
        if this.dry_run {
            this.interactor
                .log(&format!("[n] auto {} {}", service, action))
                .await;
            return Ok(());
        }
        assert_result!(user.auto(service, action).await, &this.interactor);
        Ok(())
    }
}
