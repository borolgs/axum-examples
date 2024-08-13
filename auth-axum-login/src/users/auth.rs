use std::sync::Arc;

use crate::db::{self, DB};
use rusqlite::{named_params, Row};

use super::*;

#[derive(Clone)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub access_token: Option<String>,
    pub password: Option<String>,
    pub oauth_provider: Option<String>,
    pub role: UserRole,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("email", &self.email)
            .field("role", &self.role)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .field("access_token", &"[redacted]")
            .finish()
    }
}

impl<'a> TryFrom<&Row<'a>> for User {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'a>) -> Result<Self, Self::Error> {
        let id: uuid::Uuid = row.get(0)?;

        Ok(Self {
            id: row.get(0)?,
            email: row.get(1)?,
            role: row.get(2)?,
            password: None,
            oauth_provider: None,
            access_token: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LoginUserParameters {
    pub user_email: String,
    pub access_token: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetUserByEmailParameters {
    pub user_email: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetUserByIdParameters {
    pub user_id: UserId,
}

pub type GetUserResponse = User;
pub type LoginUserResponse = User;

pub async fn login(db: DB, args: LoginUserParameters) -> db::Result<LoginUserResponse> {
    let user = db
        .call(move |conn| {
            conn.query_row(
                r#"INSERT INTO users (email, access_token) VALUES (:email, :access_token)
                    ON CONFLICT(email) DO UPDATE SET access_token=:access_token
                    RETURNING id, email, role, access_token, created_at, updated_at"#,
                named_params! {
                    ":email": args.user_email,
                    ":access_token": args.access_token
                },
                |r| User::try_from(r),
            )
            .map_err(|e| e.into())
        })
        .await?;

    Ok(user)
}

pub async fn find_one_by_id(db: DB, args: GetUserByIdParameters) -> db::Result<GetUserResponse> {
    let user_id = args.user_id;
    let user = db
        .call(move |conn| {
            conn.query_row_and_then(
                "SELECT id, email, role, access_token, created_at, updated_at FROM users WHERE id = ?",
                [args.user_id],
                |r| User::try_from(r),
            )
            .map_err(|e| e.into())
        })
        .await
        .map_err(db::Error::from)
        .map_err(|e| e.not_found_message(format!("User '{}' not found", user_id)))?;

    Ok(user)
}

pub async fn find_one_by_email(db: DB, args: GetUserByEmailParameters) -> db::Result<GetUserResponse> {
    let user_email = args.user_email.to_owned();
    let user = db
        .call(|conn| {
            conn.query_row(
                "SELECT id, email, role, access_token, created_at, updated_at FROM users WHERE email = ?",
                [args.user_email],
                |r| User::try_from(r),
            )
            .map_err(|e| e.into())
        })
        .await
        .map_err(db::Error::from)
        .map_err(|e| e.not_found_message(format!("User '{}' not found", user_email)))?;

    Ok(user)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::{self, init_test_db};

    use super::*;

    #[tokio::test]
    async fn auth_create() {
        let db = init_test_db().await.unwrap();
        let user = login(
            db,
            LoginUserParameters {
                user_email: "test@mail.com".into(),
                access_token: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(user.email, "test@mail.com");
    }

    #[tokio::test]
    async fn auth_update_token() {
        let db = init_test_db().await.unwrap();

        db.call(|conn| {
            conn.execute(
                "INSERT INTO users (email, role, access_token) VALUES ('test@mail.com', 'admin', 'prev')",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();

        let user = login(
            db,
            LoginUserParameters {
                user_email: "test@mail.com".into(),
                access_token: Some("new".into()),
            },
        )
        .await
        .unwrap();

        assert_eq!(user.email, "test@mail.com");
        assert_eq!(user.role, UserRole::Admin);
        assert_eq!(user.access_token, Some("new".into()));
    }

    #[tokio::test]
    async fn auth_get_by_id() {
        let db = init_test_db().await.unwrap();
        let user = login(
            db.clone(),
            LoginUserParameters {
                user_email: "test@mail.com".into(),
                access_token: None,
            },
        )
        .await
        .unwrap();

        let user = find_one_by_id(db, GetUserByIdParameters { user_id: user.id })
            .await
            .unwrap();

        assert_eq!(user.email, "test@mail.com");
    }

    #[tokio::test]
    async fn auth_get_by_email() {
        let db = init_test_db().await.unwrap();

        login(
            db.clone(),
            LoginUserParameters {
                user_email: "test@mail.com".into(),
                access_token: None,
            },
        )
        .await
        .unwrap();

        let user = find_one_by_email(
            db,
            GetUserByEmailParameters {
                user_email: "test@mail.com".into(),
            },
        )
        .await
        .unwrap();

        assert_eq!(user.email, "test@mail.com");
    }

    #[tokio::test]
    async fn auth_not_found() {
        let db = init_test_db().await.unwrap();

        let user = find_one_by_email(
            db.clone(),
            GetUserByEmailParameters {
                user_email: "test@mail.com".into(),
            },
        )
        .await;

        assert!(user.is_err());
        assert!(matches!(user.err(), Some(db::Error::NotFound(_))));

        let user = find_one_by_id(
            db,
            GetUserByIdParameters {
                user_id: Uuid::new_v4(),
            },
        )
        .await;

        assert!(user.is_err());
        assert!(matches!(user.err(), Some(db::Error::NotFound(_))));
    }
}
