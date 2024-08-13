use std::fmt::Display;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type UserId = Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Member,
    Guest,
}

impl Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

impl FromSql for UserRole {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_str().and_then(|v| {
            serde_json::from_str::<UserRole>(&format!("\"{}\"", v)).map_err(|_| FromSqlError::InvalidType)
        })
    }
}

pub mod auth;
