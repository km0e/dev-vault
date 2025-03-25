use crate::util::Os;

#[derive(Debug, Clone, Default)]
pub struct Params {
    pub user: String,
    pub os: Os,
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
}
