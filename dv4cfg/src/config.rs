use crate::task::config::*;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use group::TaskGroupConfig;
use serde::{Deserialize, Serialize};

pub use task::*;
use tracing::info;
use user::{LocalDeviceConfig, SSHDeviceConfig};
use xcfg::XCfg;

pub fn default_mount() -> PathBuf {
    "~/.config/dv".into()
}
pub fn default_hid() -> String {
    "local".into()
}
pub fn default_uid() -> String {
    "this".into()
}

mod group;
pub use group::Cite;
mod task;
mod user;
pub use user::UserComplete;
mod example;
pub use example::example;

use crate::{
    adapter::{GroupParts, TaskParts},
    op::ContextImpl,
    task::{Plan, Target},
    user::{UserFilter, UserManager},
};

#[derive(Default, Debug, Serialize, Deserialize, XCfg)]
struct FullConfig {
    pub id: Option<String>,
    pub ssh: Vec<SSHDeviceConfig>,
    pub local: Option<LocalDeviceConfig>,
    #[serde(default)]
    pub group: Vec<TaskGroupConfig>,
    #[serde(default)]
    pub auto: Vec<AutoTaskConfig>,
    #[serde(default)]
    pub copy: Vec<CopyTaskConfig>,
    #[serde(default)]
    pub app: Vec<AppTaskConfig>,
    #[serde(default)]
    pub exec: Vec<ExecTaskConfig>,
}

pub struct Config<I: ContextImpl> {
    pub id: Option<String>,
    ssh: Vec<SSHDeviceConfig>,
    local: Option<LocalDeviceConfig>,
    group: HashMap<String, GroupParts<I>>,
    tasks: HashMap<String, TaskParts<I>>,
}

impl<I: ContextImpl> Config<I> {
    pub fn new<P: AsRef<Path>>(path: P, dry_run: bool) -> Result<Self, xcfg::Error> {
        let fc = FullConfig::load(path)?.into_inner();
        let mut tasks = HashMap::new();
        for auto in fc.auto {
            let t = auto.cast(dry_run, &Target::default());
            tasks.insert(t.id.clone(), t);
        }
        for copy in fc.copy {
            let t = copy.cast(dry_run, &Target::default());
            tasks.insert(t.id.clone(), t);
        }
        for app in fc.app {
            let t = app.cast(dry_run, &Target::default());
            tasks.insert(t.id.clone(), t);
        }
        for exec in fc.exec {
            let t = exec.cast(dry_run, &Target::default());
            tasks.insert(t.id.clone(), t);
        }
        let group = fc
            .group
            .into_iter()
            .map(|g| {
                info!("find group {}", g.id);
                let g = g.cast(dry_run);
                (g.id().clone(), g)
            })
            .collect();
        Ok(Self {
            id: fc.id,
            local: fc.local,
            ssh: fc.ssh,
            group,
            tasks,
        })
    }
    pub async fn cast(&self, this_dir: PathBuf, id: Option<&str>) -> (UserManager, Vec<Plan<I>>) {
        match id {
            Some(id) => info!("cast group {}", id),
            None => info!("cast all group"),
        }
        let mut filter = UserFilter::default();
        let plans = match id {
            Some(id) => {
                if let Some(g) = self.group.get(id) {
                    vec![g
                        .cast(&self.group, &self.tasks, &mut filter)
                        .expect("can't cast group")]
                } else {
                    Vec::new()
                }
            }
            None => self
                .group
                .values()
                .map(|g| {
                    g.cast(&self.group, &self.tasks, &mut filter)
                        .expect("can't cast group")
                })
                .collect(),
        };
        let mut dm = UserManager::default();
        if let Some(mut dev) = self.local.clone() {
            dev.user.mount = this_dir;
            dm.extend(Some(dev.cast(&filter).await));
        }
        let mut ssh_dev = Vec::with_capacity(self.ssh.len());
        for dev in self.ssh.clone() {
            ssh_dev.push(dev.cast(&filter).await);
        }
        dm.extend(ssh_dev);
        (dm, plans)
    }
}
