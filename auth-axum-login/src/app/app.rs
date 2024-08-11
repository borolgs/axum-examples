use axum::{Extension, Router};
use axum_macros::FromRef;

use minijinja::Environment;

use tokio::net::TcpListener;
use tokio_rusqlite::Connection;
use tower_http::trace::TraceLayer;
use tower_sessions::{
    cookie::{time::Duration, SameSite},
    Expiry, SessionManagerLayer,
};
use tower_sessions_rusqlite_store::RusqliteStore;

use crate::{
    auth,
    db::{self, DB},
    notes,
    views::Views,
};

use super::{errors, state::AppState};

pub async fn create_app(db: DB) -> errors::Result<Router> {
    let session_store = RusqliteStore::new(db.clone());
    session_store.migrate().await.map_err(db::Error::from)?;
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(1)));

    let mut env = Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);
    minijinja_contrib::add_to_environment(&mut env);
    minijinja_embed::load_templates!(&mut env);

    let views = Views::new(env);
    let state = AppState {
        conn: db.clone(),
        views,
    };

    let app = Router::new()
        .merge(auth::router(state.clone()))
        .merge(notes::router(state.clone()))
        .layer(Extension(db));

    let app = auth::add_auth_layer(app, session_layer, state.conn.clone());

    Ok(app)
}
