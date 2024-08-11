use axum::async_trait;
use axum_login::AuthUser;
use oauth2::{basic::BasicClient, reqwest::async_http_client, AuthorizationCode, CsrfToken, Scope, TokenResponse};
use reqwest::{
    header::{ACCEPT, USER_AGENT},
    Url,
};
use serde::Deserialize;

use crate::{
    db::DB,
    users::{
        auth::{find_one_by_id, login, GetUserByIdParameters, LoginUserParameters, User},
        UserId,
    },
};

use super::{create_github_oauth_client, get_github_user_email, Error};

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub code: String,
    pub old_state: CsrfToken,
    pub state: CsrfToken,
}

impl AuthUser for User {
    type Id = UserId;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.access_token.as_ref().map(|token| token.as_bytes()).unwrap_or(&[])
    }
}

#[derive(Clone)]
pub struct AuthBackend {
    db: DB,
    github_oauth_client: BasicClient,
    http_client: reqwest::Client,
}

impl AuthBackend {
    pub fn new(db: DB) -> Self {
        let github_oauth_client = create_github_oauth_client();

        let http_client = {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(ACCEPT, "application/json".parse().unwrap());
            headers.insert(USER_AGENT, "axum-login".parse().unwrap());

            reqwest::Client::builder().default_headers(headers).build().unwrap()
        };

        Self {
            db,
            github_oauth_client,
            http_client,
        }
    }

    pub fn authorize_url(&self) -> (Url, CsrfToken) {
        self.github_oauth_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("user:email".to_string()))
            .url()
    }
}

#[async_trait]
impl axum_login::AuthnBackend for AuthBackend {
    type User = User;
    type Credentials = Credentials;
    type Error = Error;

    async fn authenticate(&self, creds: Self::Credentials) -> Result<Option<Self::User>, Self::Error> {
        if creds.old_state.secret() != creds.state.secret() {
            return Err(Self::Error::CsrfValidationFailed);
        }

        let token = self
            .github_oauth_client
            .exchange_code(AuthorizationCode::new(creds.code.clone()))
            .request_async(async_http_client)
            .await
            .map_err(|err| match err {
                oauth2::RequestTokenError::ServerResponse(res) => Error::RequestToken(res.error().clone()),
                err => Error::Unexpected(err.into()),
            })?;

        let access_token = token.access_token().secret().clone();

        let user_email = get_github_user_email(&self.http_client, &access_token)
            .await?
            .ok_or(Error::NoEmail)?;

        let user = login(
            self.db.clone(),
            LoginUserParameters {
                user_email: user_email.clone(),
                access_token: Some(access_token),
            },
        )
        .await?;

        tracing::info!("{user_email} logged in");

        Ok(Some(user))
    }

    async fn get_user(&self, user_id: &axum_login::UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let user = find_one_by_id(
            self.db.clone(),
            GetUserByIdParameters {
                user_id: user_id.to_owned(),
            },
        )
        .await?;

        Ok(Some(user))
    }
}

impl<AuthBackend> From<axum_login::Error<AuthBackend>> for Error
where
    AuthBackend: axum_login::AuthnBackend<Error = Error>,
{
    fn from(error: axum_login::Error<AuthBackend>) -> Self {
        match error {
            axum_login::Error::Session(err) => Error::Session(err),
            axum_login::Error::Backend(err) => err,
        }
    }
}

pub type AuthSession = axum_login::AuthSession<AuthBackend>;
