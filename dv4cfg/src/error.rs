use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(context(false), display("{source}"))]
    Api { source: dv_api::Error },
    #[snafu(whatever, display("{message}"))]
    Whatever { message: String },
    #[snafu(display("Io: {source}"))]
    Io {
        source: std::io::Error,
        about: String,
    },
    #[snafu(context(false), display("{source}"))]
    Db { source: rusqlite::Error },
}

pub type Result<T> = std::result::Result<T, Error>;
