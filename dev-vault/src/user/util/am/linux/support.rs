use super::dev;

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
