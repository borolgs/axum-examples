use rusqlite::{params, Row};
use sea_query::{Iden, Query, SqliteQueryBuilder};
use uuid::Uuid;

use crate::{ctx::BaseParams, db, Error, Result};

use super::{CreateNote, FindNotesResponse, UpdateNote};

use super::{Note, UpdateNoteForm};

#[derive(Iden)]
pub enum Notes {
    Table,
    Id,
    Title,
    Text,
    CreatedBy,
    CreatedAt,
    UpdatedBy,
    UpdatedAt,
}

impl<'a> TryFrom<&Row<'a>> for Note {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'a>) -> std::result::Result<Self, Self::Error> {
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

pub async fn find_notes(BaseParams { db, ctx }: BaseParams) -> Result<FindNotesResponse> {
    db.call(move |conn| {
        let (sql, values) = Query::select()
            .columns({
                use Notes::*;
                [Id, Title, Text, CreatedBy, CreatedAt, UpdatedBy, UpdatedAt]
            })
            .from(Notes::Table)
            .build(SqliteQueryBuilder);

        let notes = conn.prepare(
            "SELECT id, title, text, created_at, created_by, updated_at, updated_by FROM notes WHERE created_by = ?",
        )?.query_map(params![ctx.get_user_id()], |row| Note::try_from(row))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(FindNotesResponse { results: notes })
    })
    .await
    .map_err(db::Error::from)
    .map_err(Error::from)
}

pub async fn create_note(CreateNote { title, text }: CreateNote, BaseParams { db, ctx }: BaseParams) -> Result<Note> {
    db.call(move |conn| {
        conn.query_row(
            r#"INSERT INTO notes (title, text, created_by) VALUES (?, ?, ?)
            RETURNING id, title, text, created_at, created_by, updated_at, updated_by"#,
            params![title, text, ctx.get_user_id()],
            |row| Note::try_from(row),
        )
        .map_err(|e| e.into())
    })
    .await
    .map_err(db::Error::from)
    .map_err(|e| db::Error::not_found_message(e, "Note not found"))
    .map_err(Error::from)
}

pub async fn get_note(note_id: Uuid, BaseParams { db, ctx }: BaseParams) -> Result<Note> {
    db.call(move |conn| {
        let note = conn.query_row(
            "SELECT id, title, text, created_at, created_by, updated_at, updated_by FROM notes WHERE id = ?",
            params![note_id],
            |row| Note::try_from(row),
        )?;
        Ok(note)
    })
    .await
    .map_err(db::Error::from)
    .map_err(|e| db::Error::not_found_message(e, "Note not found"))
    .map_err(Error::from)
}

pub async fn update_note(
    note_id: Uuid,
    UpdateNote { text, title }: UpdateNote,
    BaseParams { db, ctx }: BaseParams,
) -> Result<Note> {
    db.call(move |conn| {
        conn.query_row(
            r#"UPDATE notes SET text = coalesce(?, text), title = coalesce(?, title), updated_at = ?, updated_by = ?
            WHERE id = ?
            RETURNING id, title, text, created_at, created_by, updated_at, updated_by"#,
            params![text, title, chrono::Utc::now(), ctx.clone().get_user_id(), note_id,],
            |row| Note::try_from(row),
        )
        .map_err(|e| e.into())
    })
    .await
    .map_err(db::Error::from)
    .map_err(|e| db::Error::not_found_message(e, "Note not found"))
    .map_err(Error::from)
}

pub async fn delete_note(note_id: Uuid, BaseParams { db, ctx }: BaseParams) -> Result<Note> {
    db.call(move |conn| {
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
    .map_err(|e| db::Error::not_found_message(e, "Note not found"))
    .map_err(Error::from)
}

pub mod views {
    use super::*;

    pub async fn get_or_create_note(note_id: Option<Uuid>, BaseParams { db, ctx }: BaseParams) -> Result<Note> {
        db.call(move |conn| {
            let note = if let Some(note_id) = note_id {
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
        .map_err(|e| db::Error::not_found_message(e, "Note not found"))
        .map_err(Error::from)
    }

    pub async fn update_note(
        UpdateNoteForm { note_id, text, title }: UpdateNoteForm,
        BaseParams { db, ctx }: BaseParams,
    ) -> Result<Note> {
        db.call(move |conn| {
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
        .map_err(|e| db::Error::not_found_message(e, "Note not found"))
        .map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ctx::{BaseParams, Ctx},
        db::init_test_db,
        Result,
    };

    #[tokio::test]
    async fn test_find_notes() -> Result<()> {
        let db = init_test_db().await?;
        let notes = find_notes(BaseParams {
            ctx: Ctx::new(None),
            db,
        });
        Ok(())
    }
}
