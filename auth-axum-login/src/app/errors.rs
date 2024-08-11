use axum::{http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::db;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not_found")]
    NotFound(String),
    #[error(transparent)]
    DB(db::Error),
    #[error("unexpected")]
    Unexpected(String),
}

impl From<db::Error> for Error {
    fn from(error: db::Error) -> Self {
        match error {
            db::Error::NotFound(msg) => Self::NotFound(msg),
            error => Self::DB(error),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum ErrorResponse {
    Unexpected { message: String },
    NotFound { message: String },
}

impl From<Error> for ErrorResponse {
    fn from(error: Error) -> Self {
        tracing::error!("{:?}", error);
        match error {
            Error::DB(_) => Self::Unexpected {
                message: "Unexpected error".into(),
            },
            Error::Unexpected(message) => Self::Unexpected { message },
            Error::NotFound(message) => Self::NotFound { message },
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let mut res = axum::Json(ErrorResponse::from(self)).into_response();
        *res.status_mut() = status;
        res
    }
}
