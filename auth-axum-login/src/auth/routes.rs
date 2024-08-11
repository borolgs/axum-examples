use axum::{
    extract::Query,
    response::{IntoResponse, Redirect},
    routing::get,
    Form, Router,
};
use minijinja::context;
use oauth2::CsrfToken;
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    auth::{Error, GithubOAuthResponse, Result},
    state::AppState,
    views::Views,
};

use super::backend::{AuthSession, Credentials};

#[derive(Deserialize)]
pub struct Next {
    pub next: Option<String>,
}

pub fn router(state: AppState) -> Router<()> {
    Router::new()
        .route("/auth/login", get(login_view).post(login))
        .route("/auth/logout", get(logout))
        .route("/auth/github/authorized", get(github_authorized))
        .with_state(state)
}

pub async fn login_view(view: Views) -> impl IntoResponse {
    view.response("login.html", context! {})
}

pub async fn login(
    auth_session: AuthSession,
    session: Session,
    Query(Next { next: query_next }): Query<Next>,
    Form(Next { next }): Form<Next>,
) -> Result<Redirect> {
    let (auth_url, csrf_token) = auth_session.backend.authorize_url();

    session
        .insert("oauth.csrf", csrf_token.secret())
        .await
        .map_err(|e| Error::Unexpected(e.into()))?;

    let next = next.or(query_next);
    if next.is_some() {
        _ = session.insert("oauth.next", next).await;
    }

    _ = session.save().await;

    Ok(Redirect::to(auth_url.as_ref()))
}

pub async fn github_authorized(
    session: Session,
    mut auth_session: AuthSession,
    Query(GithubOAuthResponse { code, state }): Query<GithubOAuthResponse>,
) -> Result<Redirect> {
    let old_state = session
        .get::<CsrfToken>("oauth.csrf")
        .await
        .map_err(|e| Error::Unexpected(e.into()))?
        .ok_or(Error::CsrfValidationFailed)?;

    let creds = Credentials { code, state, old_state };

    let user = auth_session.authenticate(creds).await?.ok_or(Error::Unauthorized)?;

    auth_session.login(&user).await?;

    if let Ok(Some(next)) = session.remove::<String>("oauth.next").await {
        Ok(Redirect::to(&next))
    } else {
        Ok(Redirect::to("/"))
    }
}

pub async fn logout(mut auth_session: AuthSession) -> Result<Redirect> {
    auth_session.logout().await?;
    Ok(Redirect::to("/"))
}
