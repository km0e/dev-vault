use snafu::Snafu;
use tokio::sync::oneshot;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(whatever, display("{message}"))]
    Whatever { message: String },
    #[snafu(display("Io: {source}"))]
    Io {
        source: std::io::Error,
        about: String,
    },
    #[snafu(context(false), display("SysTime: {source}"))]
    SysTime { source: std::time::SystemTimeError },
    #[snafu(context(false), display("Dbus: {source}"))]
    Dbus { source: zbus::Error },
    #[snafu(display("OneShot: id {id}, {source}"))]
    OneShot {
        source: oneshot::error::RecvError,
        id: String,
    },
    #[snafu(context(false), display("Parse: {source}"))]
    Parse { source: russh_config::Error },
    #[snafu(context(false), display("SSHKey: {source}"))]
    SSHKey { source: russh_keys::Error },
    #[snafu(context(false), display("SSH: {source}"))]
    SSH { source: russh::Error },
    #[snafu(display("SFTP: {source}"))]
    SFTP {
        source: russh_sftp::client::error::Error,
        about: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
