use std::path::PathBuf;

use super::{default_hid, default_mount, default_uid};
use async_trait::async_trait;
use dev_vault::{
    user::{self, User, UserCast, UserFilter},
    Environment,
};
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait UserComplete {
    async fn cast(self, filter: &UserFilter) -> (Option<User>, Vec<User>);
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
    async fn cast(self, filter: &UserFilter) -> (Option<User>, Vec<User>) {
        let mut vec = Vec::new();
        for user in self.users {
            if filter.contains(&user.uid) {
                let mut user_cfg = user::SSHUserConfig::new(user.uid, self.hid.clone(), user.host);
                user_cfg.os = self.os.clone();
                user_cfg.passwd = user.passwd;
                vec.push(user_cfg.cast().await.expect("cast user"));
            }
        }
        let system = match self.system {
            Some(system) if filter.contains(&system.uid) => {
                let mut system_cfg =
                    user::SSHUserConfig::new(system.uid, self.hid.clone(), system.host);
                system_cfg.is_system = true;
                system_cfg.os = self.os.clone();
                system_cfg.passwd = system.passwd;
                Some(system_cfg.cast().await.expect("cast system"))
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
    async fn cast(self, filter: &UserFilter) -> (Option<User>, Vec<User>) {
        let env = Environment::detect();
        let user = if filter.contains(&self.user.uid) {
            vec![user::LocalConfig {
                uid: self.user.uid,
                hid: self.hid.clone(),
                mount: self.user.mount,
            }
            .cast()
            .await
            .expect("cast user")]
        } else {
            vec![]
        };
        let system = match self.system {
            Some(system) if filter.contains(&system.uid) => {
                let mut system_cfg =
                    user::SSHUserConfig::new(system.uid, self.hid.clone(), system.host);
                system_cfg.is_system = true;
                system_cfg.passwd = system.passwd;
                system_cfg.os = env.os;
                Some(system_cfg.cast().await.expect("cast system"))
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
    pub mount: PathBuf,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            uid: default_uid(),
            mount: default_mount(),
        }
    }
}
