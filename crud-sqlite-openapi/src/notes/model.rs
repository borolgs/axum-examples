use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type UserId = Uuid;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub text: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub created_by: Option<UserId>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_by: Option<UserId>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateNoteForm {
    pub note_id: Uuid,
    pub text: String,
    pub title: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateNote {
    pub text: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateNote {
    pub text: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindNotesResponse {
    pub results: Vec<Note>,
}
