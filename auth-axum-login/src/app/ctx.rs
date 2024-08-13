use axum::{
    async_trait,
    extract::{Extension, FromRequestParts, Request},
    http::{request::Parts, HeaderMap},
    middleware::Next,
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use serde::Serialize;

use crate::{
    auth::{self, AuthSession},
    db::DB,
    users::{UserId, UserRole},
};

#[derive(Clone, Debug, FromRequestParts)]
pub struct BaseParams {
    pub ctx: Ctx,
    #[from_request(via(Extension))]
    pub db: DB,
}

impl BaseParams {
    pub fn new(db: DB, ctx: Ctx) -> Self {
        Self { db, ctx }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub role: UserRole,
}

#[derive(Clone, Debug)]
pub struct Ctx {
    pub user: Option<User>,
}

impl Ctx {
    pub fn new(user: Option<User>) -> Self {
        Self { user }
    }

    pub fn get_user_id(&self) -> Option<UserId> {
        self.user.as_ref().map(|u| u.id)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Ctx
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .extract::<AuthSession>()
            .await
            .map_err(|e| e.into_response())?
            .user
            .map(|u| User {
                id: u.id,
                email: u.email,
                role: u.role,
            });

        Ok(Self { user })
    }
}

#[derive(Clone)]
pub struct ReqCtx {
    pub headers: HeaderMap,
    pub user: Option<User>,
}

tokio::task_local! {
    pub static REQ_CTX: ReqCtx;
}

pub async fn with_ctx(headers: HeaderMap, ctx: Ctx, request: Request, next: Next) -> crate::Result<Response> {
    Ok(REQ_CTX
        .scope(
            ReqCtx {
                headers,
                user: ctx.user,
            },
            next.run(request),
        )
        .await)
}
