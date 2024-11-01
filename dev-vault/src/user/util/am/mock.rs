use snafu::whatever;

use super::dev::*;

#[derive(Default)]
pub struct MockAm {}

impl From<MockAm> for BoxedAm {
    fn from(value: MockAm) -> Self {
        Box::new(value)
    }
}

#[async_trait::async_trait]
impl Am for MockAm {
    async fn install(&self, _dev: &User, _package: &[String]) -> crate::Result<BoxedPtyProcess> {
        whatever!("other unimplemented")
    }
}
