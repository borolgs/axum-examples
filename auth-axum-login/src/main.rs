#![allow(unused)]
mod auth;
mod config;
mod db;
mod errors;
mod migrations;
mod notes;
mod state;
mod users;
mod views;

use axum::Router;
use axum_macros::FromRef;
use db::init_db;

use minijinja::Environment;

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
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "auth_axum_login=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let conn = init_db().await?;

    let session_store = RusqliteStore::new(conn.clone());
    session_store.migrate().await.map_err(db::Error::from)?;
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(1)));

    let mut env = Environment::new();
    minijinja_embed::load_templates!(&mut env);

    let views = Views::new(env);
    let state = AppState {
        conn: conn.clone(),
        views,
    };

    let app = Router::new()
        .merge(auth::router(state.clone()))
        .merge(notes::router(state.clone()))
        .layer(TraceLayer::new_for_http());

    let app = auth::add_auth_layer(app, session_layer, state.conn.clone());

    let listener = TcpListener::bind(format!("127.0.0.1:4000")).await.unwrap();

    tracing::info!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
