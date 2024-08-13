use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::users::UserId;

#[derive(Serialize, Debug)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub text: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub created_by: Option<UserId>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_by: Option<UserId>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNote {
    pub note_id: Uuid,
    pub text: String,
    pub title: String,
}
