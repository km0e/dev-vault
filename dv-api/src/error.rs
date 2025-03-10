use resplus::define_error;
use strum::EnumIs;
use thiserror::Error;

#[derive(Error, Debug, EnumIs)]
pub enum ErrorSource {
    #[error("ssh config error: {0}")]
    SSHConfig(#[from] russh_config::Error),
    #[error("ssh error: {0}")]
    SSH(#[from] russh::Error),
    #[cfg(not(windows))]
    #[error("zbus error: {0}")]
    Systemd(#[from] zbus::Error),
    #[error("sftp error: {0}")]
    SFTP(#[from] russh_sftp::client::error::Error),
    #[error("ssh key error: {0}")]
    SSHKey(#[from] russh::keys::Error),
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[error("unknown error: {0}")]
    Unknown(String),
}

#[cfg(not(windows))]
define_error!(
    ErrorSource,
    russh::Error,
    zbus::Error,
    russh_config::Error,
    russh_sftp::client::error::Error,
    russh::keys::Error,
    std::io::Error
);

#[cfg(windows)]
define_error!(
    ErrorSource,
    russh::Error,
    russh_config::Error,
    russh_sftp::client::error::Error,
    russh::keys::Error,
    std::io::Error
);
impl ErrorChain {
    pub fn is_not_found(&self) -> bool {
        if let ErrorSource::IO(ref e) = self.0.source {
            e.kind() == std::io::ErrorKind::NotFound
        } else {
            matches!(
                self.0.source,
                ErrorSource::SFTP(russh_sftp::client::error::Error::Status(
                    russh_sftp::protocol::Status {
                        status_code: russh_sftp::protocol::StatusCode::NoSuchFile,
                        ..
                    },
                ))
            )
        }
    }
}

pub type Result<T, E = ErrorChain> = std::result::Result<T, E>;

#[macro_export]
macro_rules! whatever {
    ($($t:tt)*) => {
        Err($crate::error::ErrorSource::Unknown(format!($($t)*)))?
    };
}

pub use whatever;
