mod config;

mod app;
mod ctx;
mod db;
mod errors;
mod notes;
mod openapi;
mod state;

use std::net::SocketAddr;

use aide::axum::ApiRouter;
use app::AppParams;
use axum::body::Body;
pub use config::config;
pub use db::{init_db, DB};
pub use errors::{Error, Result};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::trace::{self, TraceLayer};
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> errors::Result<()> {
    let config = config();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crud_sqlite_openapi=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .with(console_subscriber::spawn())
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_target(false),
        )
        .try_init()
        .ok();

    let conn = init_db().await?;

    let (app, api) = app::create(AppParams {
        db: conn,
        router: |state| ApiRouter::new().merge(notes::router(state)),
    })
    .await?;

    let app = app.layer(
        ServiceBuilder::new().layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().include_headers(false))
                .make_span_with(|request: &axum::http::Request<Body>| {
                    let headers = request.headers();
                    let request_id = headers
                        .get("x-request-id")
                        .map(|v| v.to_str().unwrap_or_default())
                        .unwrap_or_default();
                    let method = request.method().to_string();
                    tracing::span!(
                        tracing::Level::DEBUG,
                        "request",
                        method = method,
                        request_id = request_id,
                        uri = request.uri().to_string(),
                    )
                })
                .on_request(trace::DefaultOnRequest::new())
                .on_response(trace::DefaultOnResponse::new().include_headers(false))
                .on_failure(trace::DefaultOnFailure::new()),
        ),
    );

    let port = config.port;
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await.unwrap();

    tracing::info!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use crate::{
        app::{create, AppParams},
        config::config_override,
        errors::Result,
        state::AppState,
        DB,
    };
    use aide::axum::ApiRouter;
    use axum_test::{TestServer, TestServerConfig};

    pub async fn test_server<R>(db: DB, router: R) -> Result<TestServer>
    where
        R: FnOnce(AppState) -> ApiRouter,
    {
        config_override(|mut config| {
            // TODO
            config
        });

        let (app, _) = create(AppParams { db, router }).await?;

        let config = TestServerConfig::builder()
            .save_cookies()
            .expect_success_by_default()
            .mock_transport()
            .build();

        Ok(TestServer::new_with_config(app, config).unwrap())
    }
}
