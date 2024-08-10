use std::{convert::Infallible, sync::Arc};

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    response::{Html, IntoResponse, Response},
};
use minijinja::{Environment, Error};

#[derive(Debug, Clone)]
pub struct Views {
    pub env: Arc<Environment<'static>>,
}

impl Views {
    pub fn new(env: Environment<'static>) -> Self {
        let engine = Arc::new(env);
        Self { env: engine }
    }
}

impl Views {
    pub fn response<D: serde::Serialize>(&self, key: &str, data: D) -> Response {
        match self.render(key.as_ref(), data) {
            Ok(x) => Html(x).into_response(),
            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
        }
    }

    fn render<D: serde::Serialize>(&self, key: &str, data: D) -> Result<String, Error> {
        if key.contains('#') {
            let parts = key.split("#").collect::<Vec<&str>>();
            let template_name = parts.first();
            let block_name = parts.last();

            if let (Some(template_name), Some(block_name)) = (template_name, block_name) {
                let template = self.env.get_template(template_name)?;
                let rendered = template.eval_to_state(&data)?.render_block(&block_name)?;

                return Ok(rendered);
            }
        }

        let template = self.env.get_template(key)?;
        let rendered = template.render(&data)?;

        Ok(rendered)
    }
}

#[async_trait]
impl<ApplicationState> FromRequestParts<ApplicationState> for Views
where
    Self: FromRef<ApplicationState>,
    ApplicationState: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        _: &mut Parts,
        state: &ApplicationState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self::from_ref(state))
    }
}
