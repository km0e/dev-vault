mod local;
use std::sync::Arc;

mod ssh;

use dev::{BoxedUser, into_boxed_user};
into_boxed_user!(local::This, ssh::SSHSession);

mod dev {
    pub use super::super::Config;
    pub use super::super::dev::*;
    pub use super::super::device::Dev;
}

impl dev::Config {
    pub async fn connect(mut self, dev: Option<Arc<dev::Dev>>) -> dev::Result<dev::User> {
        if let Some(host) = self.remove("HOST") {
            ssh::create(host, self, dev).await
        } else {
            local::create(self, dev).await
        }
    }
}
