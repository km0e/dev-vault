use std::{collections::HashMap, path::Path};

use dv_api::{
    fs::{CheckInfo, DirInfo, Metadata, OpenFlags},
    process::{Interactor, Script},
    User, UserCast,
};
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
                return None;
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
                return None;
            }
        }
    };
    ($r:expr, $interactor:expr) => {
        match $r {
            Ok(v) => v,
            Err(e) => {
                let _s = e.to_string();
                $interactor.log(&_s).await;
                return None;
            }
        }
    };
}

macro_rules! value2 {
    ($t:ty, $v:expr) => {
        rune::from_value::<$t>($v).map_err(|e| e.to_string())
    };
    ($v:expr) => {
        rune::from_value::<String>($v).map_err(|e| e.to_string())
    };
}

macro_rules! obj_take {
    ($t:ty, $o:expr, $k:expr) => {
        $o.remove($k)
            .ok_or_else(|| format!("{} not found", $k))
            .and_then(|v| value2!($t, v))
    };
    ($o:expr, $k:expr) => {
        $o.remove($k)
            .ok_or_else(|| format!("{} not found", $k))
            .and_then(|v| value2!(v))
    };
}

macro_rules! field {
    ($interactor:expr, $o:expr, $k:ident@) => {
        assert_result!(obj_take!($o, stringify!($k)), $interactor)
    };
    ($interactor:expr, $o:expr, $k:ident@$t:ty) => {
        assert_result!(obj_take!($t, $o, stringify!($k)), $interactor)
    };
    ($interactor:expr, $o:expr, $k:ident@$v:expr) => {
        $v,
    };
    ($interactor:expr, $o:expr, _@default) => {
    };
}
macro_rules! obj2 {
    ($st:ident, $interactor:expr, $o:expr, $($k:ident$(@$t:tt)?),+ $(, @$d:ident)?) => {
        $st {
            $(
                $k: field!($interactor, $o, $k@$($t)?).into(),
            )+
            $(
                ..Default::$d()
            )?
        }
    };
}

impl Dv {
    #[rune::function(path = Self::add_local)]
    async fn add_current(
        mut this: Mut<Self>,
        id: Ref<str>,
        mut user: Mut<runtime::Object>,
    ) -> Option<()> {
        use dv_api::LocalConfig;
        let user = obj2!(LocalConfig, &this.interactor, user, hid, mount);
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
        let user = obj2!(SSHConfig, &this.interactor, user, hid, host, is_system@bool, @default);
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
        this.interactor
            .log(&format!("add ssh user: {}", id.as_ref()))
            .await;
        Some(())
    }
    async fn get_user(&self, uid: impl AsRef<str>) -> Option<&User> {
        Some(assert_option!(
            self.users.get(uid.as_ref()),
            &self.interactor,
            || format!("user {} not found", uid.as_ref())
        ))
    }
    async fn check_copy_file(
        &self,
        src: &User,
        src_uid: &str,
        src_path: &str,
        dst_uid: &str,
        dst_path: &str,
        ts: u64,
    ) -> Option<bool> {
        let dst = self.get_user(dst_uid).await?;
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
        Some(res)
    }

    async fn check_copy_dir(
        &self,
        src: &User,
        src_uid: &str,
        src_path: impl Into<String>,
        dst_uid: &str,
        dst_path: impl Into<String>,
        meta: Vec<Metadata>,
    ) -> Option<bool> {
        let mut src_path = src_path.into();
        let mut dst_path = dst_path.into();
        let src_len = src_path.len();
        let dst_len = dst_path.len();
        let mut success = false;
        for Metadata { path, ts } in meta {
            src_path.truncate(src_len);
            src_path.push_str(&path);
            dst_path.truncate(dst_len);
            dst_path.push_str(&path);
            let res = self
                .check_copy_file(src, src_uid, &src_path, dst_uid, &dst_path, ts)
                .await?;
            success |= res;
        }
        Some(success)
    }

    #[rune::function(path = Self::copy)]
    async fn copy(
        this: Ref<Self>,
        src_uid: Ref<str>,
        src_path: Ref<str>,
        dst_uid: Ref<str>,
        dst_path: Ref<str>,
    ) -> Option<bool> {
        assert_bool!(src_path.len() != 0, &this.interactor, || {
            "src_path is empty"
        });
        assert_bool!(dst_path.len() != 0, &this.interactor, || {
            "dst_path is empty"
        });
        let src_uid = src_uid.as_ref();
        let dst_uid = dst_uid.as_ref();
        let src_path = src_path.as_ref();
        let mut dst_path = dst_path.to_string();
        let src = this.get_user(src_uid).await?;
        if src_path.ends_with('/') {
            let DirInfo { path, files } =
                assert_result!(src.check_dir(src_path).await, &this.interactor);
            if !dst_path.ends_with('/') {
                dst_path.push('/');
            }
            this.check_copy_dir(src, src_uid, path, dst_uid, dst_path, files)
                .await
        } else {
            let info = assert_result!(src.check_path(src_path).await, &this.interactor);
            if dst_path.ends_with('/') {
                dst_path.push_str(
                    src_path
                        .rsplit_once('/')
                        .map(|(_, name)| name)
                        .unwrap_or(src_path),
                );
            };
            match info {
                CheckInfo::Dir(dir) => {
                    dst_path.push('/');
                    this.check_copy_dir(src, src_uid, dir.path, dst_uid, dst_path, dir.files)
                        .await
                }
                CheckInfo::File(file) => {
                    let Metadata { path, ts } = assert_option!(
                        file.into(),
                        &this.interactor,
                        || format!("src file {} not found", src_path)
                    );
                    this.check_copy_file(src, src_uid, &path, dst_uid, &dst_path, ts)
                        .await
                }
            }
        }
    }
    #[rune::function(path = Self::exec)]
    async fn exec(
        this: Ref<Self>,
        uid: Ref<str>,
        shell: Option<Ref<str>>,
        commands: Ref<str>,
    ) -> Option<bool> {
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
            return Some(true);
        }
        let mut pp = assert_result!(user.exec(script).await, &this.interactor);

        let ec = assert_result!(this.interactor.ask(&mut pp).await, &this.interactor);
        assert_bool!(ec == 0, &this.interactor, || "exec failed");
        Some(true)
    }
    #[rune::function(path = Self::app)]
    async fn app(this: Ref<Self>, uid: Ref<str>, package: runtime::Vec) -> Option<bool> {
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
                acc.push_str(&value2!(n)?);
                Ok(acc)
            });
        let packages = assert_result!(res, &this.interactor);
        if this.dry_run {
            this.interactor.log(&format!("[n] app {}", packages)).await;
            return Some(true);
        }
        let res = assert_result!(
            user.app(&this.interactor, &packages).await,
            &this.interactor
        );
        Some(res)
    }
    #[rune::function(path = Self::auto)]
    async fn auto(
        this: Ref<Self>,
        uid: Ref<str>,
        service: Ref<str>,
        action: Ref<str>,
    ) -> Option<bool> {
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
            return Some(true);
        }
        assert_result!(user.auto(service, action).await, &this.interactor);
        Some(true)
    }
}
