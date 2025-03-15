use crate::util::Os;

#[derive(Debug, Clone, Default)]
pub struct Params {
    pub user: String,
    pub os: Os,
    pub session: Option<String>,
    pub mount: Option<String>,
}

impl Params {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            ..Default::default()
        }
    }
    pub fn os(&mut self, os: impl Into<Os>) -> &mut Self {
        self.os = os.into();
        self
    }
    pub fn session(&mut self, session: impl Into<String>) -> &mut Self {
        self.session = Some(session.into());
        self
    }
    pub fn mount(&mut self, mount: impl Into<String>) -> &mut Self {
        self.mount = Some(mount.into());
        self
    }
}
