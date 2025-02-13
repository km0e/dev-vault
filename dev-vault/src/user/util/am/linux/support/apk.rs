use crate::user::util::am::into_boxed_am;

use super::dev::*;

#[derive(Default)]
pub struct Apk {}

#[async_trait::async_trait]
impl Am for Apk {
    async fn install(&self, u: &User, package: &[String]) -> crate::Result<BoxedPtyProcess> {
        use std::iter::once;
        let args = format!("pkgs=\"{}\"; noconfirm=t;", package.join(" "));
        let input = once(args.as_str()).chain(once(include_str!("apk.sh")));
        let cmd = Script::Script {
            program: "sh",
            input: Box::new(input),
        };
        u.exec(cmd).await
    }
}

into_boxed_am!(Apk);
