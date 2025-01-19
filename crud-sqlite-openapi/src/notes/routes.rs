use crate::{
    ctx::BaseParams,
    openapi::{
        aide::{
            axum::{routing::get, ApiRouter, IntoApiResponse},
            NoApi,
        },
        Json, Path,
    },
    state::AppState,
};
use axum::http::StatusCode;

use schemars::JsonSchema;

use serde::Deserialize;
use uuid::Uuid;

use super::{CreateNote, Note, UpdateNote};

use super::handlers;

#[derive(Debug, Deserialize, JsonSchema)]
struct NoteIdPath {
    note_id: Uuid,
}

pub fn router(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/api/v1/notes",
            get(find_notes).post_with(create_note, |t| t.response::<201, Json<Note>>()),
        )
        .api_route(
            "/api/v1/notes/{note_id}",
            get(get_note).patch(update_note).delete(delete_note),
        )
        .with_state(state)
}

async fn find_notes(NoApi(base): NoApi<BaseParams>) -> impl IntoApiResponse {
    handlers::find_notes(base).await.map(Json)
}

async fn create_note(NoApi(base): NoApi<BaseParams>, Json(args): Json<CreateNote>) -> impl IntoApiResponse {
    handlers::create_note(args, base)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
}

async fn get_note(
    Path(NoteIdPath { note_id }): Path<NoteIdPath>,
    NoApi(base): NoApi<BaseParams>,
) -> impl IntoApiResponse {
    handlers::get_note(note_id, base).await.map(Json)
}

async fn update_note(
    Path(NoteIdPath { note_id }): Path<NoteIdPath>,
    NoApi(base): NoApi<BaseParams>,
    Json(args): Json<UpdateNote>,
) -> impl IntoApiResponse {
    handlers::update_note(note_id, args, base).await.map(Json)
}

async fn delete_note(
    Path(NoteIdPath { note_id }): Path<NoteIdPath>,
    NoApi(base): NoApi<BaseParams>,
) -> impl IntoApiResponse {
    handlers::delete_note(note_id, base).await.map(Json)
}

#[cfg(test)]
mod tests {
    use crate::{
        db::{init_test_db, DB},
        errors::Result,
        notes::{FindNotesResponse, Note},
    };
    use axum_test::TestServer;
    use serde_json::json;

    #[tokio::test]
    async fn find_notes() -> Result<()> {
        let db = init_test_db().await?;

        db.call(|conn| {
            conn.execute_batch(
                r#"
                INSERT INTO notes (title, text, created_by) VALUES ('first', '1', uuid_blob('018f6146-32f4-7948-8289-cfb5cdb2b2af'));
                INSERT INTO notes (title, text, created_by) VALUES ('second', '2', uuid_blob('018f6146-32f4-7948-8289-cfb5cdb2b2af'));
                INSERT INTO notes (title, text, created_by) VALUES ('third', '3', uuid_blob('018f6146-32f4-7948-8289-cfb5cdb2b2af'));
                "#,
            )
            .unwrap();
            Ok(())
        })
        .await
        .unwrap();

        let server = test_server(db).await?;
        let response = server.get("/api/v1/notes").await;

        assert_eq!(response.status_code(), 200);
        assert_eq!(response.json::<FindNotesResponse>().results.len(), 3);
        Ok(())
    }

    #[tokio::test]
    async fn create_note() -> Result<()> {
        let db = init_test_db().await?;

        let server = test_server(db).await?;
        let response = server
            .post("/api/v1/notes")
            .json(&json!({
                "text": "hello",
                "title": "world"
            }))
            .await;

        assert_eq!(response.status_code(), 201);
        assert_eq!(response.json::<Note>().title, "world");
        Ok(())
    }

    #[tokio::test]
    async fn get_note() -> Result<()> {
        let db = init_test_db().await?;

        db.call(|conn| {
            conn.execute_batch(
                "INSERT INTO notes (id, title, text) VALUES (uuid_blob('018f6138-5b4f-722d-97c5-29b927cedbd4'), 'first', '1');",
            )
            .unwrap();
            Ok(())
        })
        .await
        .unwrap();

        let server = test_server(db).await?;
        let response = server.get("/api/v1/notes/018f6138-5b4f-722d-97c5-29b927cedbd4").await;

        assert_eq!(response.status_code(), 200);
        assert_eq!(response.json::<Note>().title, "first");
        Ok(())
    }

    #[tokio::test]
    async fn update_note() -> Result<()> {
        let db = init_test_db().await?;

        db.call(|conn| {
            conn.execute_batch(
                "INSERT INTO notes (id, title, text) VALUES (uuid_blob('018f6138-5b4f-722d-97c5-29b927cedbd4'), 'first', '1');",
            )
            .unwrap();
            Ok(())
        })
        .await
        .unwrap();

        let server = test_server(db).await?;
        let response = server
            .patch("/api/v1/notes/018f6138-5b4f-722d-97c5-29b927cedbd4")
            .json(&json!({
                "text": "2",
            }))
            .await;

        assert_eq!(response.status_code(), 200);
        assert_eq!(response.json::<Note>().title, "first");
        assert_eq!(response.json::<Note>().text, "2");
        Ok(())
    }

    #[tokio::test]
    async fn delete_note() -> Result<()> {
        let db = init_test_db().await?;

        db.call(|conn| {
            conn.execute(
                "INSERT INTO notes (id, title, text) VALUES (uuid_blob('018f6138-5b4f-722d-97c5-29b927cedbd4'), 'first', '1');",
                []
            )
            .unwrap();
            Ok(())
        })
        .await
        .unwrap();

        let server = test_server(db.clone()).await?;
        let response = server
            .delete("/api/v1/notes/018f6138-5b4f-722d-97c5-29b927cedbd4")
            .await;

        assert_eq!(response.status_code(), 200);
        assert_eq!(response.json::<Note>().title, "first");

        let count = db
            .call(|conn| {
                conn.query_row::<u32, _, _>("select count(*) from notes", [], |r| r.get(0))
                    .map_err(|e| e.into())
            })
            .await
            .unwrap();

        assert_eq!(count, 0);

        Ok(())
    }

    async fn test_server(db: DB) -> Result<TestServer> {
        crate::tests::test_server(db, super::router).await
    }
}
