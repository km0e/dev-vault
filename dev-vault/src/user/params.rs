use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Params {
    pub user: String,
    pub os: String,
    pub session: Option<String>,
    pub home: Option<PathBuf>,
    pub mount: Option<PathBuf>,
}

impl Params {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            os: "unspecified".to_string(),
            session: None,
            home: None,
            mount: None,
        }
    }
    pub fn os_str(mut self, os: impl AsRef<str>) -> Self {
        self.os.clear();
        self.os.push_str(os.as_ref());
        self
    }
    pub fn os(mut self, os: impl Into<String>) -> Self {
        self.os = os.into();
        self
    }
    pub fn session(mut self, session: impl Into<String>) -> Self {
        self.session = Some(session.into());
        self
    }
    pub fn home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }
    pub fn mount(mut self, mount: impl Into<PathBuf>) -> Self {
        let mount: PathBuf = mount.into();
        let mount = match (self.home.as_ref(), mount.strip_prefix("~")) {
            (Some(h), Ok(p)) => h.join(if p.has_root() {
                p.strip_prefix("/").unwrap()
            } else {
                p
            }),
            (None, Ok(_)) => panic!("we need home"),
            _ => mount,
        };
        self.mount = Some(mount);
        self
    }
}
