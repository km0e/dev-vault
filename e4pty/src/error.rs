use resplus::define_error;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[cfg(not(windows))]
    #[error("openpty error: {0}")]
    Errno(#[from] rustix_openpty::rustix::io::Errno),
    #[error("unknown error: {0}")]
    Unknown(String),
}

#[cfg(not(windows))]
define_error!(Error, std::io::Error, rustix_openpty::rustix::io::Errno);

#[cfg(windows)]
define_error!(Error, std::io::Error);

pub type Result<T, E = ErrorChain> = std::result::Result<T, E>;
