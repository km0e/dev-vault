use std::sync::Arc;

use tracing::{info, warn};

use super::{This, dev::*};

fn detect() -> Os {
    if cfg!(target_os = "linux") {
        etc_os_release::OsRelease::open()
            .inspect_err(|e| warn!("can't open [/etc/os-release | /usr/lib/os-release]: {}", e))
            .map(|os_release| os_release.id().into())
            .unwrap_or("linux".into())
    } else if cfg!(target_os = "macos") {
        "macos".into()
    } else if cfg!(target_os = "windows") {
        "windows".into()
    } else {
        "unknown".into()
    }
}

#[cfg(windows)]
fn is_user_admin() -> bool {
    use windows_sys::Win32::Security::{
        AllocateAndInitializeSid, CheckTokenMembership, FreeSid, SECURITY_NT_AUTHORITY,
    };
    use windows_sys::Win32::System::SystemServices::{
        DOMAIN_ALIAS_RID_ADMINS, SECURITY_BUILTIN_DOMAIN_RID,
    };

    unsafe {
        let mut sid = std::ptr::null_mut();
        let success = AllocateAndInitializeSid(
            &SECURITY_NT_AUTHORITY,
            2,
            SECURITY_BUILTIN_DOMAIN_RID as u32,
            DOMAIN_ALIAS_RID_ADMINS as u32,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut sid,
        ) != 0;

        if !success {
            return false;
        }

        let mut is_member = 0;
        let check_success = CheckTokenMembership(std::ptr::null_mut(), sid, &mut is_member) != 0;

        FreeSid(sid);

        check_success && is_member != 0
    }
}
#[cfg(not(windows))]
fn is_user_admin() -> bool {
    rustix::process::getuid().is_root()
}

pub async fn create(mut cfg: Config, dev: Option<Arc<Dev>>) -> Result<User> {
    let is_system = cfg.is_system.unwrap_or_else(is_user_admin);
    if let Some(session) = {
        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_SESSION_TYPE").ok()
        }
        #[cfg(target_os = "macos")]
        {
            None::<String>
        }
        #[cfg(target_os = "windows")]
        {
            None::<String>
        }
    } {
        cfg.insert("SESSION", session);
    }
    if let Some(user) = {
        #[cfg(target_os = "linux")]
        {
            std::env::var("USER").ok()
        }
        #[cfg(target_os = "macos")]
        {
            None
        }
        #[cfg(target_os = "windows")]
        {
            std::env::var("USERNAME").ok()
        }
    } {
        cfg.insert("USER", user);
    }

    let u: BoxedUser = This::new(is_system).await?.into();
    let dev = match dev {
        Some(dev) => dev,
        None => {
            let os = detect();
            info!("detect os:{}", os);
            let pm = Pm::new(&u, &os).await?;
            Arc::new(Dev { pm, os })
        }
    };
    User::new(cfg.vars, is_system, u, dev).await
}
