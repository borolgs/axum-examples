use axum::{http::StatusCode, response::IntoResponse, Json};
use oauth2::basic::BasicErrorResponseType;
use serde::Serialize;
use serde_json::Value;
use tower_sessions::session;

use crate::db;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("csrf_validation_failed")]
    CsrfValidationFailed,
    #[error("request_token")]
    RequestToken(BasicErrorResponseType),

    #[error("no_email")]
    NoEmail,

    #[error(transparent)]
    DB(#[from] db::Error),

    #[error(transparent)]
    Session(#[from] session::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    pub error: String,
    pub message: Option<String>,
    #[serde(default)]
    pub details: Option<Value>,
}

impl ErrorResponse {
    pub fn new(error: String, message: Option<String>, details: Option<Value>) -> Self {
        Self {
            error,
            message,
            details,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("unauthorized".into(), None, None)),
            ),
            Error::Forbidden => (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("forbidden".into(), None, None)),
            ),
            Error::CsrfValidationFailed => (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("csrf_validation_failed".into(), None, None)),
            ),
            Error::RequestToken(token_error) => {
                let error = token_error.as_ref().to_owned();
                match token_error {
                    BasicErrorResponseType::InvalidClient => {
                        (StatusCode::FORBIDDEN, Json(ErrorResponse::new(error, None, None)))
                    }
                    BasicErrorResponseType::InvalidGrant => {
                        (StatusCode::FORBIDDEN, Json(ErrorResponse::new(error, None, None)))
                    }
                    BasicErrorResponseType::InvalidRequest => {
                        (StatusCode::BAD_REQUEST, Json(ErrorResponse::new(error, None, None)))
                    }
                    BasicErrorResponseType::InvalidScope => {
                        (StatusCode::FORBIDDEN, Json(ErrorResponse::new(error, None, None)))
                    }
                    BasicErrorResponseType::UnauthorizedClient => {
                        (StatusCode::UNAUTHORIZED, Json(ErrorResponse::new(error, None, None)))
                    }
                    BasicErrorResponseType::UnsupportedGrantType => {
                        (StatusCode::BAD_REQUEST, Json(ErrorResponse::new(error, None, None)))
                    }
                    BasicErrorResponseType::Extension(error) => {
                        (StatusCode::BAD_REQUEST, Json(ErrorResponse::new(error, None, None)))
                    }
                }
            }
            err => {
                tracing::error!("{err:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("Unexpected error".into(), None, None)),
                )
            }
        }
        .into_response()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
