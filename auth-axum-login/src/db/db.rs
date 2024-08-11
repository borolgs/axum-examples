use rusqlite::functions::FunctionFlags;
use tokio_rusqlite::Connection;
use uuid::Uuid;

use crate::config::config;

use super::migrations::MIGRATIONS;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not_found")]
    NotFound(String),
    #[error(transparent)]
    TokioRusqlite(tokio_rusqlite::Error),
    #[error(transparent)]
    Rusqlite(rusqlite::Error),
}

impl Error {
    pub fn not_found_message(self, message: impl Into<String>) -> Self {
        if (matches!(self, Self::NotFound(_))) {
            return Self::NotFound(message.into());
        }
        self
    }
}

impl From<tokio_rusqlite::Error> for Error {
    fn from(error: tokio_rusqlite::Error) -> Self {
        match error {
            tokio_rusqlite::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => Self::NotFound("Not found".into()),
            error => Self::TokioRusqlite(error),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::Rusqlite(error)
    }
}

pub type DB = Connection;

pub async fn init_db() -> Result<DB> {
    let conn = tokio_rusqlite::Connection::open(&config().database_url).await?;

    conn.call(|conn| {
        add_uuid_functions(conn)?;

        MIGRATIONS.to_latest(conn).unwrap();

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        Ok(())
    })
    .await?;

    Ok(conn)
}

#[cfg(test)]
pub async fn init_test_db() -> Result<DB> {
    let conn = tokio_rusqlite::Connection::open_in_memory().await?;

    conn.call(|conn| {
        add_uuid_functions(conn)?;

        MIGRATIONS.to_latest(conn).unwrap();

        Ok(())
    })
    .await?;

    Ok(conn)
}

fn add_uuid_functions(conn: &mut rusqlite::Connection) -> rusqlite::Result<()> {
    conn.create_scalar_function("uuid7_now", 0, FunctionFlags::SQLITE_UTF8, |_| Ok(Uuid::now_v7()))?;

    conn.create_scalar_function("uuid_blob", 1, FunctionFlags::SQLITE_UTF8, |ctx| {
        let value = ctx.get::<String>(0)?;
        let uuid = Uuid::parse_str(&value).map_err(|e| rusqlite::Error::UserFunctionError(e.into()))?;

        Ok(uuid)
    })?;

    Ok(())
}
