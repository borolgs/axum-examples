#![allow(unused_variables)]

use axum::{Router, http::StatusCode, response::IntoResponse, routing::post};
use axum_macros::FromRef;
use serde::Serialize;
use tokio::net::TcpListener;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unexpected")]
    Unexpected(String),
}

#[derive(Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum ErrorResponse {
    Unexpected { message: String },
}

impl From<Error> for ErrorResponse {
    fn from(error: Error) -> Self {
        match error {
            Error::Unexpected(message) => Self::Unexpected { message },
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let mut res = axum::Json(ErrorResponse::from(self)).into_response();
        *res.status_mut() = status;
        res
    }
}

async fn execute_script() -> impl IntoResponse {
    "hello"
}

#[derive(FromRef, Clone)]
pub struct AppState {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/run", post(execute_script))
        .with_state(AppState {});

    let listener = TcpListener::bind(format!("127.0.0.1:4000")).await?;

    println!("listening on http://{}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
