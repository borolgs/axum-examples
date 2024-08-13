#![allow(unused)]
mod app;
mod auth;
mod db;
mod notes;
mod shared;
mod users;

use axum::Router;
use axum_macros::FromRef;
use db::init_db;

use minijinja::Environment;

pub use app::{
    config, create_app, ctx,
    errors::{self, Error, Result},
    state,
};
pub use shared::views;

use state::AppState;
use tokio::net::TcpListener;
use tokio_rusqlite::Connection;
use tower_http::trace::TraceLayer;
use tower_sessions::{
    cookie::{time::Duration, SameSite},
    Expiry, SessionManagerLayer,
};
use tower_sessions_rusqlite_store::RusqliteStore;
use tracing_subscriber::prelude::*;
use views::Views;

#[tokio::main]
async fn main() -> errors::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");

    shared::tracing::setup_tracing(false);

    let conn = init_db().await?;

    let app = create_app(conn).await?;

    let app = shared::tracing::add_tracing_layer(app);

    let listener = TcpListener::bind(format!("127.0.0.1:4000")).await.unwrap();

    tracing::info!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
