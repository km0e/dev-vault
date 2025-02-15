use crate::user::util::am::into_boxed_am;

use super::dev::*;

#[derive(Default)]
pub struct Pacman {}

#[async_trait::async_trait]
impl Am for Pacman {
    async fn install(&self, u: &User, packages: &str) -> crate::Result<BoxedPtyProcess> {
        use std::iter::once;
        let args = format!("am=pacman; pkgs=\"{}\"; noconfirm=t;", packages);
        let input = once(args.as_str()).chain(once(include_str!("pacman.sh")));
        let cmd = Script::Script {
            program: "sh",
            input: Box::new(input),
        };
        u.exec(cmd).await
    }
}
into_boxed_am!(Pacman);
