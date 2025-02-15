use crate::user::util::am::into_boxed_am;

use super::dev::*;

#[derive(Default)]
pub struct Paru {}

#[async_trait::async_trait]
impl Am for Paru {
    async fn install(&self, u: &User, packages: &str) -> crate::Result<BoxedPtyProcess> {
        use std::iter::once;
        let args = format!("am=paru; pkgs=\"{}\"; noconfirm=t;", packages);
        let input = once(args.as_str()).chain(once(include_str!("pacman.sh")));
        let cmd = Script::Script {
            program: "sh",
            input: Box::new(input),
        };
        u.exec(cmd).await
    }
}

into_boxed_am!(Paru);
