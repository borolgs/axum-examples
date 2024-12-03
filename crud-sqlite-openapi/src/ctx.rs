use axum::{
    async_trait,
    extract::{Extension, FromRequestParts, Request},
    http::{request::Parts, HeaderMap},
    middleware::Next,
    response::Response,
};
use schemars::JsonSchema;
use serde::Serialize;
use uuid::{uuid, Uuid};

use crate::DB;

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

#[derive(Debug, Serialize, Clone, JsonSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
}

#[derive(Clone, Debug)]
pub struct Ctx {
    pub user: Option<User>,
}

impl Ctx {
    pub fn new(user: Option<User>) -> Self {
        Self { user }
    }

    pub fn get_user_id(&self) -> Option<Uuid> {
        self.user.as_ref().map(|u| u.id)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Ctx
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self {
            // TODO
            user: Some(User {
                id: uuid!("018f6146-32f4-7948-8289-cfb5cdb2b2af"),
                email: "fake@mail.com".into(),
            }),
        })
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
