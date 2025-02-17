use super::dev::{self, *};
use crate::util::am::into_boxed_am;

mod apk;
pub use apk::Apk;
mod apt;
pub use apt::Apt;
mod pacman;
pub use pacman::Pacman;
mod yay;
pub use yay::Yay;
mod paru;
pub use paru::Paru;

into_boxed_am!(Apk, Apt, Pacman, Yay, Paru);
