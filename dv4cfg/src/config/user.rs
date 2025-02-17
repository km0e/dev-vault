use crate::user::UserFilter;

use super::{default_hid, default_mount, default_uid};
use async_trait::async_trait;
use dv_api::{SSHConfig, User, UserCast};
use serde::{Deserialize, Serialize};
use tracing::info;

#[async_trait]
pub trait UserComplete {
    async fn cast(self, filter: &UserFilter) -> (Option<(String, User)>, Vec<(String, User)>);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSHUserConfig {
    pub uid: String,
    pub host: String,
    pub passwd: Option<String>,
}

impl SSHUserConfig {
    pub fn new(uid: impl Into<String>, host: impl Into<String>) -> Self {
        Self {
            uid: uid.into(),
            host: host.into(),
            passwd: None,
        }
    }
    pub fn passwd(mut self, pw: impl Into<String>) -> Self {
        self.passwd = Some(pw.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSHDeviceConfig {
    pub hid: String,
    pub os: Option<String>,
    system: Option<SSHUserConfig>,
    #[serde(default)]
    users: Vec<SSHUserConfig>,
}

#[async_trait]
impl UserComplete for SSHDeviceConfig {
    async fn cast(self, filter: &UserFilter) -> (Option<(String, User)>, Vec<(String, User)>) {
        let mut vec = Vec::new();
        for user in self.users {
            if filter.contains(&user.uid) {
                let mut user_cfg = SSHConfig::new(self.hid.clone(), user.host);
                user_cfg.os = self.os.clone();
                user_cfg.passwd = user.passwd;
                vec.push((user.uid, user_cfg.cast().await.expect("cast user")));
            }
        }
        let system = match self.system {
            Some(system) if filter.contains(&system.uid) => {
                let mut system_cfg = SSHConfig::new(self.hid.clone(), system.host);
                system_cfg.is_system = true;
                system_cfg.os = self.os.clone();
                system_cfg.passwd = system.passwd;
                Some((system.uid, system_cfg.cast().await.expect("cast system")))
            }
            _ => None,
        };

        (system, vec)
    }
}

impl SSHDeviceConfig {
    pub fn new(id: impl Into<String>, users: impl IntoIterator<Item = SSHUserConfig>) -> Self {
        Self {
            hid: id.into(),
            os: None,
            system: None,
            users: users.into_iter().collect(),
        }
    }
    pub fn root(mut self, root: SSHUserConfig) -> Self {
        self.system = Some(root);
        self
    }
    pub fn os(mut self, os: impl Into<String>) -> Self {
        self.os = Some(os.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDeviceConfig {
    #[serde(default = "default_hid")]
    pub hid: String,
    pub user: LocalConfig,
    pub system: Option<SSHUserConfig>,
}

#[async_trait]
impl UserComplete for LocalDeviceConfig {
    async fn cast(self, filter: &UserFilter) -> (Option<(String, User)>, Vec<(String, User)>) {
        let user = if filter.contains(&self.user.uid) {
            info!("cast user {}", self.user.uid);
            vec![(
                self.user.uid,
                dv_api::LocalConfig {
                    hid: self.hid.clone(),
                    mount: self.user.mount,
                }
                .cast()
                .await
                .expect("cast user"),
            )]
        } else {
            vec![]
        };
        let system = match self.system {
            Some(system) if filter.contains(&system.uid) => {
                let mut system_cfg = SSHConfig::new(self.hid.clone(), system.host);
                system_cfg.is_system = true;
                system_cfg.passwd = system.passwd;
                Some((system.uid, system_cfg.cast().await.expect("cast system")))
            }
            _ => None,
        };

        (system, user)
    }
}

impl Default for LocalDeviceConfig {
    fn default() -> Self {
        Self {
            hid: default_hid(),
            user: LocalConfig::default(),
            system: None,
        }
    }
}

impl LocalDeviceConfig {
    pub fn system(mut self, system: SSHUserConfig) -> Self {
        self.system = Some(system);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    #[serde(default = "default_uid")]
    pub uid: String,
    #[serde(default = "default_mount")]
    pub mount: camino::Utf8PathBuf,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            uid: default_uid(),
            mount: default_mount(),
        }
    }
}
