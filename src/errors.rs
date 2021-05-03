use rocket::{http::Status, response::Responder};
use snafu::Snafu;
use std::option::NoneError;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ServerError {
    #[snafu(display("This is a test error: {}", msg))]
    TestError {
        msg: String,
    },
    #[snafu(display("Sled Error: {}", source))]
    SledError {
        source: sled_extensions::Error,
    },
    #[snafu(display("The secret associated with this domain did not match"))]
    SecretMismatch {
        domain: String,
    },
    NotFound,
}

impl From<NoneError> for ServerError {
    fn from(_: NoneError) -> Self {
        ServerError::NotFound
    }
}

impl<'a, 'b: 'a> Responder<'a, 'static> for ServerError {
    fn respond_to(self, _: &rocket::Request) -> Result<rocket::Response<'static>, Status> {
        match self {
            ServerError::SecretMismatch { domain: _ } => Err(Status::Forbidden),
            ServerError::TestError { msg: _ } => Err(Status::InternalServerError),
            _ => Err(Status::InternalServerError),
        }
    }
}
