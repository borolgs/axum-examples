mod handlers;
mod model;
mod routes;

use model::*;

use crate::{openapi::aide::axum::ApiRouter, state::AppState};

pub fn router(state: AppState) -> ApiRouter {
    ApiRouter::new().merge(routes::router(state.clone()))
}
