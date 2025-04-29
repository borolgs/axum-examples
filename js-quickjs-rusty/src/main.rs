#![allow(unused_variables)]
mod js;

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use axum_macros::FromRef;
use js::Runtime;
use serde::Serialize;
use serde_json::Value;
use tokio::net::TcpListener;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Runtime(#[from] js::Error),
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
            Error::Runtime(err) => Self::Unexpected {
                message: err.to_string(),
            },
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

async fn execute_script(
    State(runtime): State<js::Runtime>,
    Json(args): Json<js::Script>,
) -> impl IntoResponse {
    runtime
        .execute_script(args)
        .await
        .map(Json)
        .map_err(Error::from)
        .into_response()
}

async fn sum(State(runtime): State<js::Runtime>, Json(args): Json<Value>) -> impl IntoResponse {
    runtime
        .execute_script(js::Script::Function {
            args: Some(args),
            code: "console.log('a+b');args.a + args.b".into(),
        })
        .await
        .map(Json)
        .map_err(Error::from)
        .into_response()
}

#[derive(FromRef, Clone)]
pub struct AppState {
    runtime: js::Runtime,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let runtime = Runtime::new();

    let app = Router::new()
        .route("/run", post(execute_script))
        .route("/sum", post(sum))
        .with_state(AppState { runtime });

    let listener = TcpListener::bind(format!("127.0.0.1:4000")).await?;

    println!("listening on http://{}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
