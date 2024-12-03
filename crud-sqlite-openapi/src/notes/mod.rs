mod handlers;
mod model;
mod routes_api;

use model::*;

use crate::{openapi::aide::axum::ApiRouter, state::AppState};

pub fn router(state: AppState) -> ApiRouter {
    ApiRouter::new().merge(routes_api::router(state.clone()))
}
