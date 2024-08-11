mod backend;
mod errors;
mod github;
mod routes;

use axum::Router;
use axum_login::AuthManagerLayerBuilder;

use tower_sessions::SessionStore;

pub use backend::AuthSession;
pub use errors::{Error, Result};
pub use routes::router;

pub(crate) use github::*;

use crate::db::DB;

use self::backend::AuthBackend;

pub fn add_auth_layer(
    app: Router,
    session_layer: tower_sessions::SessionManagerLayer<impl SessionStore + Clone>,
    db: DB,
) -> Router {
    let auth_backend = AuthBackend::new(db);
    let auth_layer = AuthManagerLayerBuilder::new(auth_backend, session_layer).build();

    app.layer(auth_layer)
}

pub mod middleware {
    use crate::users::UserRole;
    use axum::{
        extract::Request,
        http::{StatusCode, Uri},
        middleware::Next,
        response::{IntoResponse, Redirect, Response},
    };

    use super::*;

    pub async fn protected(auth_session: AuthSession, request: Request, next: Next) -> Result<Response> {
        #[cfg(test)]
        if crate::config::config().skip_auth {
            return Ok(next.run(request).await);
        }

        auth_session.user.ok_or(Error::Unauthorized)?;
        Ok(next.run(request).await)
    }

    pub async fn protected_view(
        auth_session: AuthSession,
        url: Uri,
        request: Request,
        next: Next,
    ) -> impl IntoResponse {
        #[cfg(test)]
        if crate::config::config().skip_auth {
            return next.run(request).await;
        };

        if let Some(user) = auth_session.user {
            return next.run(request).await;
        }

        let path = url.path();
        let redirect_url = format!("/auth/login?next={path}");

        Redirect::to(&redirect_url).into_response()
    }
}
