#[derive(Debug, Clone)]
pub struct Params {
    pub user: String,
    pub os: String,
    pub session: Option<String>,
    pub mount: Option<camino::Utf8PathBuf>,
}

impl Params {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            os: "unspecified".to_string(),
            session: None,
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
    pub fn mount(mut self, mount: impl Into<camino::Utf8PathBuf>) -> Self {
        self.mount = Some(mount.into());
        self
    }
}
