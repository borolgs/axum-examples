use std::sync::{Arc, OnceLock};

use crate::error_responses;
use axum::{
    extract::{
        rejection::{PathRejection, QueryRejection},
        Request,
    },
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_jsonschema::JsonSchemaRejection;
use schemars::{
    schema::{Schema, SchemaObject, SubschemaValidation},
    schema_for, schema_for_value, JsonSchema,
};
use serde::Serialize;
use serde_json::Value;

pub use response::{ErrorResponse, ErrorResponseDocs};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not_found")]
    NotFound(String),

    // auth
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,

    // validation
    #[error("validation")]
    JsonValidation(JsonSchemaRejection),
    #[error("validation")]
    QueryValidation(#[from] QueryRejection),
    #[error("validation")]
    PathValidation(#[from] PathRejection),

    #[error(transparent)]
    DB(crate::db::Error),

    // other
    #[error(transparent)]
    /// An application-specific error.
    App(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("unexpected")]
    Unexpected(String),
}

impl From<JsonSchemaRejection> for Error {
    fn from(rejection: JsonSchemaRejection) -> Self {
        Self::JsonValidation(rejection)
    }
}

impl From<crate::db::Error> for Error {
    fn from(error: crate::db::Error) -> Self {
        match error {
            crate::db::Error::NotFound(msg) => Self::NotFound(msg),
            error => Self::DB(error),
        }
    }
}

/// crate::Error <--> tokio_rusqlite::Error
/// ```rust
/// impl From<tokio_rusqlite::Error> for Error { }
/// impl From<Error> for tokio_rusqlite::Error { }
/// ```
pub mod db_mappers {
    use super::*;
    use crate::db::rusqlite;
    use crate::db::tokio_rusqlite;

    impl From<tokio_rusqlite::Error> for Error {
        fn from(error: tokio_rusqlite::Error) -> Self {
            match error {
                tokio_rusqlite::Error::Other(err) => {
                    if err.is::<Error>() {
                        return *err.downcast::<Error>().unwrap();
                    }
                    return Error::DB(tokio_rusqlite::Error::Other(err).into());
                }
                _ => Error::DB(error.into()),
            }
        }
    }

    impl From<rusqlite::Error> for Error {
        fn from(error: rusqlite::Error) -> Self {
            Error::DB(error.into())
        }
    }

    impl From<Error> for tokio_rusqlite::Error {
        fn from(error: Error) -> Self {
            tokio_rusqlite::Error::Other(error.into())
        }
    }
}

// Response

error_responses! {
    not_found: 404,
    path_validation: 400,
    query_validation: 400,
    json_validation: 400,
    unauthorized: 401,
    forbidden: 403,
    unexpected: 500
}

impl From<&Error> for ErrorResponse {
    fn from(error: &Error) -> Self {
        let errors = errors();
        match error {
            Error::NotFound(message) => errors.not_found.with_message(message),
            Error::Unauthorized => errors.unauthorized.with_message("Unauthorized"),
            Error::Forbidden => errors.forbidden.with_message("Forbitten"),
            Error::JsonValidation(json_error) => {
                let message = match json_error {
                    JsonSchemaRejection::Json(error) => error.body_text(),
                    JsonSchemaRejection::Serde(error) => error.to_string(),
                    JsonSchemaRejection::Schema(_) => "Request schema validation error".into(), // TODO: details
                };
                errors.json_validation.with_message(message)
            }
            Error::QueryValidation(error) => errors.query_validation.with_message(error.body_text()),
            Error::PathValidation(error) => errors.path_validation.with_message(error.body_text()),
            Error::App(app_error) => {
                let msg = app_error.to_string();
                errors.unexpected.with_message(msg)
            }
            _ => errors.unexpected.with_message("Unexpected"),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let error = Arc::new(self);

        let error_res = ErrorResponse::from(error.clone().as_ref());
        let status = error_res.status;

        let mut res = axum::Json(error_res).into_response();
        res.extensions_mut().insert(error);

        *res.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        res
    }
}

pub async fn on_error(request: Request, next: Next) -> Response {
    let response = next.run(request).await;

    let error = response.extensions().get::<Arc<Error>>().map(Arc::as_ref);
    if let Some(error) = error {
        tracing::error!("{:?}", error);
    }

    response
}

mod response {
    use serde_json::Map;

    use super::*;

    #[derive(Debug, Serialize, Clone, Default, JsonSchema)]
    pub struct ErrorResponse {
        pub error: String,
        pub message: Option<String>,
        pub status: u16,
        pub details: Option<Map<String, Value>>,
    }

    impl ErrorResponse {
        pub fn new(error: impl Into<String>, status: u16) -> Self {
            Self {
                error: error.into(),
                status,
                ..Default::default()
            }
        }

        pub fn with_message(&self, message: impl Into<String>) -> Self {
            let mut res = self.clone();
            res.message = Some(message.into());
            res
        }
    }

    pub struct ErrorResponseDocs;

    impl JsonSchema for ErrorResponseDocs {
        fn schema_name() -> String {
            String::from("ErrorResponse")
        }

        fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
            let errors = errors();
            let mut example_schema = schema_for_value!(errors);

            let error_schemas = example_schema
                .schema
                .metadata()
                .examples
                .clone()
                .first()
                .map(|e| e.as_object().unwrap().values())
                .unwrap()
                .map(|v| {
                    let error = v.get("error").unwrap().as_str().unwrap().to_string();
                    let status = v.get("status").unwrap().as_u64().unwrap();

                    let mut schema = schema_for!(ErrorResponse).schema;
                    let obj = schema.object();
                    obj.properties.get_mut("status").map(|p| {
                        if let Schema::Object(obj) = p {
                            obj.enum_values = Some(vec![Value::from(status)]);
                        }
                        p
                    });
                    obj.properties.get_mut("error").map(|p| {
                        if let Schema::Object(obj) = p {
                            obj.enum_values = Some(vec![Value::from(error)]);
                        }
                        p
                    });

                    Schema::from(schema)
                })
                .collect::<Vec<_>>();

            let schema = SchemaObject {
                subschemas: Some(Box::new(SubschemaValidation {
                    one_of: Some(error_schemas),
                    ..Default::default()
                })),
                ..Default::default()
            };

            schema.into()
        }
    }

    /// Typed responses with a custom JSON schema
    /// ```rust
    /// error_responses! {
    ///     not_found: 404,
    ///     unexpected: 500
    /// }
    ///
    /// impl From<&Error> for ErrorResponse {
    ///     fn from(error: &Error) -> Self {
    ///     let errors = errors(); // <- from macro
    ///     match error {
    ///         Error::NotFound(message) => errors.not_found.with_message(message),
    ///         Error::Unexpected(message) => errors.unexpected.with_message(message),
    ///     }
    /// }
    /// ```
    #[macro_export]
    macro_rules! error_responses {
        (
            $($name:ident: $code:expr),* $(,)?
        ) => {
            #[derive(Debug, Clone, Serialize)]
            struct Responses {
                $(
                    $name: ErrorResponse,
                )*
            }

            static ERRORS: OnceLock<Responses> = OnceLock::new();

            fn errors() -> &'static Responses {
                ERRORS.get_or_init(|| Responses {
                    $(
                        $name: ErrorResponse::new(stringify!($name), $code),
                    )*
                })
            }
        };
    }
}
