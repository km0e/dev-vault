use crate::Error;

impl From<russh_config::Error> for Error {
    fn from(e: russh_config::Error) -> Self {
        Error::File {
            message: e.to_string(),
        }
    }
}

impl From<russh::keys::Error> for Error {
    fn from(e: russh::keys::Error) -> Self {
        Error::Authentication {
            message: e.to_string(),
        }
    }
}

impl From<russh::Error> for Error {
    fn from(e: russh::Error) -> Self {
        match e {
            russh::Error::CouldNotReadKey => Error::File {
                message: e.to_string(),
            },
            _ => Error::Whatever {
                message: e.to_string(),
            },
        }
    }
}
