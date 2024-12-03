use aide::scalar::Scalar;
use axum::{
    middleware::{self},
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;

use crate::config;

use rand::Rng;
use serde_json::json;

use crate::{
    ctx::with_ctx,
    db::DB,
    errors::{self, on_error, ErrorResponseDocs},
    openapi::{
        self,
        aide::axum::{ApiRouter, IntoApiResponse},
        OpenApi,
    },
    state::AppState,
};

pub struct AppParams<Router>
where
    Router: FnOnce(AppState) -> ApiRouter,
{
    pub db: DB,
    pub router: Router,
}

pub async fn create<R>(AppParams { db, router }: AppParams<R>) -> errors::Result<(Router, OpenApi)>
where
    R: FnOnce(AppState) -> ApiRouter,
{
    let mut api = OpenApi::default();

    let state = AppState { conn: db.clone() };

    let api_router = axum::Router::new().route(
        "/__docs__",
        get(Scalar::new("/__docs__/spec.json")
            .with_title("Notes API")
            .axum_handler()),
    );

    // TODO
    #[cfg(not(test))]
    let api_router = api_router.route("/__docs__/spec.json", get(serve_docs));

    let app = ApiRouter::new()
        .route("/__version__", get(version))
        .route("/__heartbeat__", get(heartbeat))
        .route("/__lbheartbeat__", get(lbheartbeat))
        .merge(api_router)
        .merge(router(state.clone()))
        .finish_api_with(&mut api, |t| {
            t.title("Notes").default_response::<openapi::Json<ErrorResponseDocs>>()
        })
        .layer(
            ServiceBuilder::new()
                .layer(Extension(db))
                .layer(Extension(Arc::new(api.clone())))
                .layer(middleware::from_fn(with_ctx))
                .layer(middleware::from_fn(on_error)),
        );

    Ok((app, api))
}

async fn version() -> impl IntoResponse {
    let config = &config();
    Json(json!({
        "source" : config.source,
        "version": config.version,
        "commit" : config.git_commit,
        "build"  : config.pipeline_id
    }))
}

async fn heartbeat() -> impl IntoResponse {
    let mut rng = rand::thread_rng();
    let random: u32 = rng.gen_range(0..=10000);

    Json(json!({
        "status" : "ok",
        "random": random,
    }))
}

async fn lbheartbeat() -> impl IntoResponse {
    ""
}

// TODO
#[cfg(not(test))]
async fn serve_docs(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    openapi::Json(api).into_response()
}
