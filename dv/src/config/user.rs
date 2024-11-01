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
                vec.push(
                    user::SSHUserConfig {
                        uid: user.uid,
                        hid: self.hid.clone(),
                        is_system: true,
                        os: self.os.clone(),
                        host: user.host,
                        passwd: user.passwd,
                    }
                    .cast()
                    .await
                    .expect("cast user"),
                );
            }
        }
        let system = match self.system {
            Some(system) if filter.contains(&system.uid) => Some(
                user::SSHUserConfig {
                    uid: system.uid,
                    hid: self.hid.clone(),
                    is_system: true,
                    os: self.os.clone(),
                    host: system.host,
                    passwd: system.passwd,
                }
                .cast()
                .await
                .expect("cast system"),
            ),
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
pub struct HostDeviceConfig {
    #[serde(default = "default_hid")]
    pub hid: String,
    pub user: HostConfig,
    system: Option<SSHUserConfig>,
}

#[async_trait]
impl UserComplete for HostDeviceConfig {
    async fn cast(self, filter: &UserFilter) -> (Option<User>, Vec<User>) {
        let env = Environment::detect();
        let user = if filter.contains(&self.user.uid) {
            vec![user::HostConfig {
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
            Some(system) if filter.contains(&system.uid) => Some(
                user::SSHUserConfig {
                    uid: system.uid,
                    hid: self.hid.clone(),
                    is_system: true,
                    os: env.os,
                    host: system.host,
                    passwd: system.passwd,
                }
                .cast()
                .await
                .expect("cast system"),
            ),
            _ => None,
        };

        (system, user)
    }
}

impl Default for HostDeviceConfig {
    fn default() -> Self {
        Self {
            hid: default_hid(),
            user: HostConfig::default(),
            system: None,
        }
    }
}

impl HostDeviceConfig {
    pub fn system(mut self, system: SSHUserConfig) -> Self {
        self.system = Some(system);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    #[serde(default = "default_uid")]
    pub uid: String,
    #[serde(default = "default_mount")]
    pub mount: PathBuf,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            uid: default_uid(),
            mount: default_mount(),
        }
    }
}
