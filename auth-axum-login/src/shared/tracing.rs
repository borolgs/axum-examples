use axum::Router;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace,
    trace::TraceLayer,
};
use tracing_subscriber::prelude::*;

use crate::config::config;

pub fn setup_tracing(json: bool) {
    let tracing = tracing_subscriber::registry().with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "auth_axum_login=debug,tower_http=debug,axum::rejection=trace".into()),
    );

    if json {
        tracing.with(tracing_subscriber::fmt::layer().json()).init();
    } else {
        tracing.with(tracing_subscriber::fmt::layer()).init();
    };
}

pub fn add_tracing_layer(app: Router) -> Router {
    app.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(trace::DefaultMakeSpan::new().include_headers(false))
                    .on_request(trace::DefaultOnRequest::new())
                    .on_response(trace::DefaultOnResponse::new().include_headers(false))
                    .on_failure(trace::DefaultOnFailure::new()),
            ),
    )
}
