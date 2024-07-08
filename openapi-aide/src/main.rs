#![allow(dead_code)]
use aide::{
    axum::{
        routing::{get, get_with},
        ApiRouter, IntoApiResponse,
    },
    openapi::OpenApi,
    OperationIo,
};

use axum::{
    extract::rejection::{PathRejection, QueryRejection},
    http::StatusCode,
    response::IntoResponse,
};
use axum_jsonschema::JsonSchemaRejection;
use axum_macros::{FromRequest, FromRequestParts};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use uuid::Uuid;

// Errors

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("validation")]
    JsonValidation(JsonSchemaRejection),
    #[error("validation")]
    QueryValidation(#[from] QueryRejection),
    #[error("validation")]
    PathValidation(#[from] PathRejection),

    #[error("unexpected")]
    Unexpected(String),
}

impl From<JsonSchemaRejection> for Error {
    fn from(rejection: JsonSchemaRejection) -> Self {
        Self::JsonValidation(rejection)
    }
}

#[derive(Serialize, JsonSchema)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum ErrorResponse {
    PathValidation {
        message: String,
    },
    QueryValidation {
        message: String,
    },
    JsonValidation {
        message: String,
        details: Option<Value>,
    },

    Unexpected {
        message: String,
    },
}

impl From<Error> for ErrorResponse {
    fn from(error: Error) -> Self {
        match error {
            Error::PathValidation(error) => Self::PathValidation {
                message: error.body_text(),
            },
            Error::QueryValidation(error) => Self::QueryValidation {
                message: error.body_text(),
            },
            Error::JsonValidation(JsonSchemaRejection::Schema(errors)) => Self::JsonValidation {
                message: "Request schema validation error".into(),
                details: Some(json!(errors)),
            },
            Error::JsonValidation(JsonSchemaRejection::Json(error)) => Self::JsonValidation {
                message: error.body_text(),
                details: None,
            },
            Error::JsonValidation(JsonSchemaRejection::Serde(error)) => Self::JsonValidation {
                message: error.to_string(),
                details: None,
            },

            Error::Unexpected(message) => Self::Unexpected { message },
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Error::PathValidation(_) | Error::QueryValidation(_) | Error::JsonValidation(_) => {
                StatusCode::BAD_REQUEST
            }
            Error::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let mut res = axum::Json(ErrorResponse::from(self)).into_response();
        *res.status_mut() = status;
        res
    }
}

// Aide newtypes

#[derive(FromRequestParts, OperationIo)]
#[from_request(via(axum::extract::Path), rejection(Error))]
#[aide(
    input_with = "axum::extract::Path<T>",
    output_with = "axum_jsonschema::Json<T>",
    json_schema
)]
pub struct Path<T>(pub T);

#[derive(FromRequestParts, OperationIo)]
#[from_request(via(axum::extract::Query), rejection(Error))]
#[aide(
    input_with = "axum::extract::Query<T>",
    output_with = "axum_jsonschema::Json<T>",
    json_schema
)]
#[aide]
pub struct Query<T>(pub T);

#[derive(FromRequest, OperationIo)]
#[from_request(via(axum_jsonschema::Json), rejection(Error))]
#[aide(
    input_with = "axum_jsonschema::Json<T>",
    output_with = "axum_jsonschema::Json<T>",
    json_schema
)]
pub struct Json<T>(pub T);

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}

// Notes API

#[derive(Deserialize, JsonSchema)]
struct CreateNote {
    title: Option<String>,
    text: String,
}

#[derive(Deserialize, JsonSchema)]
struct FindNotes {
    /// Search by text or by title
    query: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct NoteId {
    /// Uuid v7
    note_id: Uuid,
}

#[derive(Serialize, JsonSchema)]
struct Note {
    id: Uuid,
    title: String,
    text: String,
}

async fn find_note(Path(NoteId { note_id }): Path<NoteId>) -> impl IntoApiResponse {
    let note = Note {
        id: note_id,
        title: "New note".into(),
        text: "Some texxt".into(),
    };
    Json(note)
}

async fn find_notes(Query(FindNotes { query: _ }): Query<FindNotes>) -> impl IntoApiResponse {
    let notes: Vec<Note> = vec![];
    Json(notes)
}

async fn create_note(Json(args): Json<CreateNote>) -> impl IntoApiResponse {
    let note = Note {
        id: Uuid::now_v7(),
        title: args.title.unwrap_or("New note".into()),
        text: args.text,
    };

    (StatusCode::CREATED, Json(note))
}

#[tokio::main]
async fn main() {
    let mut api = OpenApi::default();

    let app = ApiRouter::new()
        .api_route(
            "/notes/:note_id",
            get_with(find_note, |t| {
                t.summary("Get a note by its ID")
                    .description("Retrieve a single note using its unique identifier.")
            }),
        )
        .api_route(
            "/notes",
            get(find_notes).post_with(create_note, |t| t.response::<201, Json<Note>>()),
        )
        .finish_api_with(&mut api, |t| {
            t.title("My Notes")
                .version("0.0.1")
                .default_response::<Json<ErrorResponse>>()
        });

    //let json = serde_json::to_string_pretty(&api).unwrap();
    let yaml = serde_yaml::to_string(&api).unwrap();
    println!("{}", yaml);

    let listener = TcpListener::bind(format!("127.0.0.1:4000")).await.unwrap();

    println!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
