use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    NotFound,
    File {
        message: String,
    },
    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
    },
    Authentication {
        message: String,
    },
    Service {
        message: String,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
