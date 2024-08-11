use std::sync::Arc;

use axum::extract::FromRef;
use tokio::sync::broadcast;

use crate::{db::DB, shared::views::Views};

#[derive(FromRef, Clone)]
pub struct AppState {
    pub conn: DB,
    pub views: Views,
}
