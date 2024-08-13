use std::{ops::Deref, sync::Arc};

use crate::{
    auth,
    ctx::BaseParams,
    db::{self, DB},
    errors::Error,
    users::UserId,
    views::Views,
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Form, Router,
};
use minijinja::{context, Environment};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::{handlers, UpdateNote};

#[derive(Debug, Deserialize)]
struct EditNoteQuery {
    note_id: Option<Uuid>,
}

async fn get_note(Path(note_id): Path<Uuid>, view: Views, base: BaseParams) -> impl IntoResponse {
    let note = handlers::get_note(note_id, base).await;

    note.map(|note| view.response("notes.html#note", context! { note => note }))
        .into_response()
}

async fn notes_view(view: Views, base: BaseParams) -> impl IntoResponse {
    let user = base.clone().ctx.user;

    let notes = handlers::find_notes(base).await;

    notes
        .map_err(|err| {
            tracing::error!("{err:?}");
            view.response(
                "notes.html",
                context! { notes => json!([]), error => err.to_string(), user => user },
            );
        })
        .map(|notes| view.response("notes.html", context! { notes => notes, user => user }))
        .into_response()
}

async fn edit_note_view(view: Views, base: BaseParams, Query(edit_query): Query<EditNoteQuery>) -> impl IntoResponse {
    let note = handlers::get_or_create_note(edit_query.note_id, base).await;

    note.map(|note| view.response("note-edit.html", context! { note => note }))
        .map_err(Error::from)
        .into_response()
}

async fn update_note(view: Views, base: BaseParams, Form(args): Form<UpdateNote>) -> impl IntoResponse {
    let user = base.ctx.clone().user;
    let note = handlers::update_note(args, base).await;

    note.map(|note| view.response("notes.html#note", context! { note => note, user => user  }))
        .map_err(Error::from)
        .into_response()
}

async fn delete_note(Path(note_id): Path<Uuid>, base: BaseParams) -> impl IntoResponse {
    let deleted_note = handlers::delete_note(note_id, base).await;

    deleted_note.map(|_| "").map_err(Error::from).into_response()
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(notes_view))
        .route("/notes/:note_id", get(get_note).delete(delete_note))
        .route("/notes", post(update_note))
        .route("/edit", get(edit_note_view))
        .with_state(state)
        .layer(middleware::from_fn(auth::middleware::protected_view))
}
