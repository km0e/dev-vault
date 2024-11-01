use std::{env, fmt::Display, path::PathBuf};

use tracing::warn;

#[derive(Debug, Clone)]
pub struct Environment {
    pub user: String,
    pub os: Option<String>,
    pub session: Option<String>,
    pub home: Option<PathBuf>,
}

impl Environment {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            os: None,
            session: None,
            home: None,
        }
    }
    pub fn os(mut self, os: impl Into<String>) -> Self {
        self.os = Some(os.into());
        self
    }
    pub fn home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OS: {:<10}, Home: {:?}, Session: {:?}",
            self.os.as_deref().unwrap_or("unspecified"),
            self.home,
            self.session
        )
    }
}

impl Environment {
    pub fn detect() -> Self {
        let user = {
            #[cfg(target_os = "linux")]
            {
                env::var("USER").unwrap_or("unspecified".to_string())
            }
            #[cfg(target_os = "macos")]
            {
                Some("macos".to_string())
            }
            #[cfg(target_os = "windows")]
            {
                Some("windows".to_string())
            }
        };

        let os = {
            #[cfg(target_os = "linux")]
            {
                Some(
                    etc_os_release::OsRelease::open()
                        .inspect_err(|e| {
                            warn!("can't open [/etc/os-release | /usr/lib/os-release]: {}", e)
                        })
                        .map(|os_release| os_release.id().to_string())
                        .unwrap_or("linux".to_string())
                        .to_string(),
                )
            }
            #[cfg(target_os = "macos")]
            {
                Some("macos".to_string())
            }
            #[cfg(target_os = "windows")]
            {
                Some("windows".to_string())
            }
        };
        let session = {
            #[cfg(target_os = "linux")]
            {
                Some(std::env::var("XDG_SESSION_TYPE").unwrap_or("unspecified".to_string()))
            }
            #[cfg(target_os = "macos")]
            {
                None
            }
            #[cfg(target_os = "windows")]
            {
                None
            }
        };
        let home = home::home_dir();
        Self {
            user,
            os,
            session,
            home,
        }
    }
}
