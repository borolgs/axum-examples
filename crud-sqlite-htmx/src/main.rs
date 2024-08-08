#![allow(unused)]
mod db;
mod errors;
mod migrations;
mod notes;
mod views;

use axum::Router;
use axum_macros::FromRef;
use db::init_db;

use minijinja::Environment;

use tokio::net::TcpListener;
use tokio_rusqlite::Connection;
use tower_http::trace::TraceLayer;
use tracing_subscriber::prelude::*;
use views::Views;

#[derive(FromRef, Clone)]
pub struct AppState {
    conn: Connection,
    views: Views,
}

#[tokio::main]
async fn main() -> errors::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crud_sqlite_htmx=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let conn = init_db().await?;

    let mut env = Environment::new();

    notes::add_templates(&mut env);

    let views = Views::new(env);
    let state = AppState { conn, views };

    let app = Router::new()
        .merge(notes::router(state))
        .layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(format!("127.0.0.1:4000")).await.unwrap();

    tracing::info!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
