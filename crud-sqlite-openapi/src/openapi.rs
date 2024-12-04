use aide::operation::OperationIo;
use aide::OperationOutput;
use axum::response::IntoResponse;
use axum_macros::{FromRequest, FromRequestParts};
use indexmap::IndexMap;
use serde::Serialize;

pub use aide;
pub use aide::openapi::OpenApi;

use crate::errors::ErrorResponseDocs;

#[derive(FromRequest, OperationIo)]
#[from_request(via(axum_jsonschema::Json), rejection(crate::Error))]
#[aide(
    input_with = "axum_jsonschema::Json<T>",
    output_with = "axum_jsonschema::Json<T>",
    json_schema
)]
pub struct Json<T>(pub T);

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}

#[derive(FromRequestParts, OperationIo)]
#[from_request(via(axum::extract::Query), rejection(crate::Error))]
#[aide(
    input_with = "axum::extract::Query<T>",
    output_with = "axum_jsonschema::Json<T>",
    json_schema
)]
#[aide]
pub struct Query<T>(pub T);

#[derive(FromRequestParts, OperationIo)]
#[from_request(via(axum::extract::Path), rejection(crate::Error))]
#[aide(
    input_with = "axum::extract::Path<T>",
    output_with = "axum_jsonschema::Json<T>",
    json_schema
)]
pub struct Path<T>(pub T);

impl OperationOutput for crate::Error {
    type Inner = ();

    fn operation_response(
        ctx: &mut aide::gen::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Option<aide::openapi::Response> {
        let mut schema = ctx.schema.subschema_for::<ErrorResponseDocs>().into_object();

        Some(aide::openapi::Response {
            description: schema.metadata().description.clone().unwrap_or_default(),
            content: IndexMap::from_iter([(
                "application/json".into(),
                aide::openapi::MediaType {
                    schema: Some(aide::openapi::SchemaObject {
                        json_schema: schema.into(),
                        example: None,
                        external_docs: None,
                    }),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        })
    }

    fn inferred_responses(
        ctx: &mut aide::gen::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<u16>, aide::openapi::Response)> {
        if let Some(res) = Self::operation_response(ctx, operation) {
            let default_response = [(None, res)];
            Vec::from(default_response)
        } else {
            Vec::new()
        }
    }
}
