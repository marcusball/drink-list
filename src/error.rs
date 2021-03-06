use actix_web::error::ResponseError;
use actix_web::Error as ActixError;
use diesel::r2d2;
use diesel::result::Error as DieselError;
use futures::channel::oneshot::Canceled as FutureCanceled;
use std::convert::From;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug, Display)]
pub enum Error {
    SessionNotFound,

    DieselError(DieselError),

    PoolError(r2d2::PoolError),

    R2D2Error(r2d2::Error),

    FutureCanceled(FutureCanceled),

    EntryInputError(String),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DieselError(e) => Some(e),
            Self::PoolError(e) => Some(e),
            Self::R2D2Error(e) => Some(e),
            Self::FutureCanceled(e) => Some(e),
            Self::SessionNotFound => None,
            Self::EntryInputError(_) => None,
        }
    }
}

impl ResponseError for Error {}

impl From<DieselError> for Error {
    fn from(e: DieselError) -> Error {
        Error::DieselError(e)
    }
}

impl From<r2d2::PoolError> for Error {
    fn from(e: r2d2::PoolError) -> Error {
        Error::PoolError(e)
    }
}

impl From<r2d2::Error> for Error {
    fn from(e: r2d2::Error) -> Error {
        Error::R2D2Error(e)
    }
}

impl From<FutureCanceled> for Error {
    fn from(e: FutureCanceled) -> Error {
        Error::FutureCanceled(e)
    }
}
