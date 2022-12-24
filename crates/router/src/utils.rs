pub(crate) mod custom_serde;
pub(crate) mod db_utils;
mod ext_traits;
mod fp_utils;

#[cfg(feature = "kv_store")]
pub(crate) mod storage_partitioning;

pub(crate) use common_utils::{
    crypto,
    ext_traits::{ByteSliceExt, BytesExt, Encode, StringExt, ValueExt},
    validation::validate_email,
};
use nanoid::nanoid;

pub(crate) use self::{
    ext_traits::{OptionExt, ValidateCall},
    fp_utils::when,
};
use crate::consts;

pub mod error_parser {
    use std::fmt::Display;

    use actix_web::{
        error::{Error, JsonPayloadError},
        http::StatusCode,
        HttpRequest, ResponseError,
    };

    #[derive(Debug)]
    struct CustomJsonError {
        err: JsonPayloadError,
    }

    // Display is a requirement defined by the actix crate for implementing ResponseError trait
    impl Display for CustomJsonError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(
                serde_json::to_string(&serde_json::json!({
                    "error": self.err.to_string()
                }))
                .as_deref()
                .unwrap_or("Invalid Json Error"),
            )
        }
    }

    impl ResponseError for CustomJsonError {
        fn status_code(&self) -> StatusCode {
            StatusCode::INTERNAL_SERVER_ERROR
        }

        fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
            use actix_web::http::header;

            actix_web::HttpResponseBuilder::new(StatusCode::BAD_REQUEST)
                .insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
                .insert_header((header::VIA, "Juspay_Router"))
                .body(self.to_string())
        }
    }

    pub(crate) fn custom_json_error_handler(err: JsonPayloadError, _req: &HttpRequest) -> Error {
        actix_web::error::Error::from(CustomJsonError { err })
    }
}

#[inline]
pub fn generate_id(length: usize, prefix: &str) -> String {
    format!("{}_{}", prefix, nanoid!(length, &consts::ALPHABETS))
}
