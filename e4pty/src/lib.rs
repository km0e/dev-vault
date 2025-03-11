pub mod prelude {
    pub use super::core::{
        BoxedPtyReader, BoxedPtyWriter, PtyReader, PtyWriter, Script, ScriptExecutor, WindowSize,
    };
    pub use super::instance::openpty_local;
}

mod core;
mod error;
mod instance;
pub use error::{Error, ErrorChain, Result};
