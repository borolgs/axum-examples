use axum::extract::FromRef;

use crate::db::DB;

#[derive(FromRef, Clone)]
pub struct AppState {
    pub conn: DB,
}
