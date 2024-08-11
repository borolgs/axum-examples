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
use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_sessions::Session;
use uuid::Uuid;

#[derive(Serialize, Debug)]
struct Note {
    id: Uuid,
    title: String,
    text: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub created_by: Option<UserId>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_by: Option<UserId>,
}

#[derive(Debug, Deserialize)]
struct EditNoteQuery {
    note_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct UpdateNote {
    note_id: Uuid,
    text: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct DeleteNote {
    note_id: Uuid,
}

impl<'a> TryFrom<&Row<'a>> for Note {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'a>) -> Result<Self, Self::Error> {
        let id: uuid::Uuid = row.get(0)?;

        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            text: row.get(2)?,
            created_at: row.get(3)?,
            created_by: row.get(4)?,
            updated_at: row.get(5)?,
            updated_by: row.get(6)?,
        })
    }
}

async fn get_note(Path(note_id): Path<Uuid>, view: Views, State(db): State<DB>) -> impl IntoResponse {
    let note = db
        .call(move |conn| {
            let note = conn.query_row(
                "SELECT id, title, text, created_at, created_by, updated_at, updated_by FROM notes WHERE id = ?",
                params![note_id],
                |row| Note::try_from(row),
            )?;
            Ok(note)
        })
        .await
        .map_err(db::Error::from)
        .map_err(|e| db::Error::not_found_message(e, "Note not found"));

    note.map(|note| view.response("notes.html#note", context! { note => note }))
        .map_err(Error::from)
        .into_response()
}

async fn notes_view(view: Views, BaseParams { db, ctx }: BaseParams) -> impl IntoResponse {
    let user = ctx.user;

    let notes = db
        .call(|conn| {
            let notes = conn
                .prepare("SELECT id, title, text, created_at, created_by, updated_at, updated_by FROM notes")?
                .query_map([], |row| Note::try_from(row))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(notes)
        })
        .await
        .map_err(db::Error::from);

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

async fn edit_note_view(
    view: Views,
    BaseParams { db, ctx }: BaseParams,
    Query(edit_query): Query<EditNoteQuery>,
) -> impl IntoResponse {
    let note = db
        .call(move |conn| {
            let note = if let Some(note_id) = edit_query.note_id {
                conn.query_row(
                    "SELECT id, title, text, created_at, created_by, updated_at, updated_by FROM notes WHERE id = ?",
                    params![note_id],
                    |row| Note::try_from(row),
                )?
            } else {
                conn.query_row(
                    r#"INSERT INTO notes (title, text, created_by) VALUES ('', '', ?)
                    RETURNING id, title, text, created_at, created_by, updated_at, updated_by"#,
                    params![ctx.get_user_id()],
                    |row| Note::try_from(row),
                )?
            };
            Ok(note)
        })
        .await
        .map_err(db::Error::from)
        .map_err(|e| db::Error::not_found_message(e, "Note not found"));

    note.map(|note| view.response("note-edit.html", context! { note => note }))
        .map_err(Error::from)
        .into_response()
}

async fn update_note(
    view: Views,
    BaseParams { db, ctx }: BaseParams,
    Form(UpdateNote { note_id, text, title }): Form<UpdateNote>,
) -> impl IntoResponse {
    let user = ctx.clone().user;
    let note = db
        .call(move |conn| {
            conn.query_row(
                r#"UPDATE notes SET text = ?, title = ?, updated_at = ?, updated_by = ?
                WHERE id = ?
                RETURNING id, title, text, created_at, created_by, updated_at, updated_by"#,
                params![text, title, chrono::Utc::now(), ctx.clone().get_user_id(), note_id],
                |row| Note::try_from(row),
            )
            .map_err(|e| e.into())
        })
        .await
        .map_err(db::Error::from)
        .map_err(|e| db::Error::not_found_message(e, "Note not found"));

    note.map(|note| view.response("notes.html#note", context! { note => note, user => user  }))
        .map_err(Error::from)
        .into_response()
}

async fn delete_note(State(db): State<DB>, Path(DeleteNote { note_id }): Path<DeleteNote>) -> impl IntoResponse {
    let deleted_note = db
        .call(move |conn| {
            conn.query_row(
                r#"DELETE FROM notes
                WHERE id = ?
                RETURNING id, title, text, created_at, created_by, updated_at, updated_by"#,
                params![note_id],
                |row| Note::try_from(row),
            )
            .map_err(|e| e.into())
        })
        .await
        .map_err(db::Error::from)
        .map_err(|e| db::Error::not_found_message(e, "Note not found"));

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
