use std::{ops::Deref, sync::Arc};

use crate::{
    db::{self, DB},
    errors::Error,
    views::Views,
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Form, Router,
};
use minijinja::{context, Environment};
use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Serialize, Debug)]
struct Note {
    id: Uuid,
    title: String,
    text: String,
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
        })
    }
}

async fn get_note(Path(note_id): Path<Uuid>, view: Views, State(db): State<DB>) -> impl IntoResponse {
    let note = db
        .call(move |conn| {
            let note = conn.query_row(
                "SELECT id, title, text, created_at, updated_at FROM notes WHERE id = ?",
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

async fn notes_view(view: Views, State(db): State<DB>) -> impl IntoResponse {
    let notes = db
        .call(|conn| {
            let notes = conn
                .prepare("SELECT id, title, text, created_at, updated_at FROM notes")?
                .query_map([], |row| Note::try_from(row))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(notes)
        })
        .await
        .map_err(db::Error::from);

    notes
        .map_err(|err| view.response("notes.html", context! { notes => json!([]), error => err.to_string() }))
        .map(|notes| view.response("notes.html", context! { notes => notes }))
        .into_response()
}

async fn edit_note_view(
    view: Views,
    State(db): State<DB>,
    Query(edit_query): Query<EditNoteQuery>,
) -> impl IntoResponse {
    let note = db
        .call(move |conn| {
            let note = if let Some(note_id) = edit_query.note_id {
                conn.query_row(
                    "SELECT id, title, text, created_at, updated_at FROM notes WHERE id = ?",
                    params![note_id],
                    |row| Note::try_from(row),
                )?
            } else {
                conn.query_row(
                    "INSERT INTO notes (title, text) VALUES ('', '') RETURNING id, title, text",
                    [],
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
    State(db): State<DB>,
    Form(UpdateNote { note_id, text, title }): Form<UpdateNote>,
) -> impl IntoResponse {
    let note = db
        .call(move |conn| {
            let note = conn.query_row(
                "UPDATE notes SET text = ?, title = ?, updated_at = ? WHERE id = ? RETURNING id, title, text",
                params![text, title, chrono::Utc::now(), note_id],
                |row| {
                    Ok(Note {
                        id: row.get(0).unwrap(),
                        title: row.get(1).unwrap(),
                        text: row.get(2).unwrap(),
                    })
                },
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

async fn delete_note(State(db): State<DB>, Path(DeleteNote { note_id }): Path<DeleteNote>) -> impl IntoResponse {
    let deleted_note = db
        .call(move |conn| {
            let note = conn.query_row(
                "DELETE FROM notes WHERE id = ? RETURNING id, title, text",
                params![note_id],
                |row| {
                    Ok(Note {
                        id: row.get(0).unwrap(),
                        title: row.get(1).unwrap(),
                        text: row.get(2).unwrap(),
                    })
                },
            )?;
            Ok(note)
        })
        .await
        .map_err(db::Error::from)
        .map_err(|e| db::Error::not_found_message(e, "Note not found"));

    deleted_note.map(|_| "").map_err(Error::from).into_response()
}

pub fn add_templates(env: &mut Environment) {
    [
        env.add_template("base.html", include_str!("views/base.html")),
        env.add_template("page.html", include_str!("views/page.html")),
        env.add_template("notes.html", include_str!("views/notes.html")),
        env.add_template("note-edit.html", include_str!("views/note-edit.html")),
    ]
    .map(|r| r.unwrap());
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(notes_view))
        .route("/notes/:note_id", get(get_note).delete(delete_note))
        .route("/notes", post(update_note))
        .route("/edit", get(edit_note_view))
        .with_state(state)
}
