pub mod custom_serde;
pub mod db_utils;
pub mod ext_traits;

#[cfg(feature = "kv_store")]
pub mod storage_partitioning;

pub use common_utils::{
    crypto,
    ext_traits::{ByteSliceExt, BytesExt, Encode, StringExt, ValueExt},
    fp_utils::when,
    validation::validate_email,
};
use error_stack::{IntoReport, ResultExt};
use nanoid::nanoid;
use serde::de::DeserializeOwned;

pub use self::ext_traits::{OptionExt, ValidateCall};
use crate::{
    consts,
    core::errors::{self, RouterResult},
    logger, types,
};

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
            StatusCode::BAD_REQUEST
        }

        fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
            use actix_web::http::header;

            actix_web::HttpResponseBuilder::new(self.status_code())
                .insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
                .body(self.to_string())
        }
    }

    pub fn custom_json_error_handler(err: JsonPayloadError, _req: &HttpRequest) -> Error {
        actix_web::error::Error::from(CustomJsonError { err })
    }
}

#[inline]
pub fn generate_id(length: usize, prefix: &str) -> String {
    format!("{}_{}", prefix, nanoid!(length, &consts::ALPHABETS))
}

pub trait ConnectorResponseExt: Sized {
    fn get_response(self) -> RouterResult<types::Response>;
    fn get_error_response(self) -> RouterResult<types::Response>;
    fn get_response_inner<T: DeserializeOwned>(self, type_name: &'static str) -> RouterResult<T> {
        self.get_response()?
            .response
            .parse_struct(type_name)
            .change_context(errors::ApiErrorResponse::InternalServerError)
    }
}

impl<E> ConnectorResponseExt
    for Result<Result<types::Response, types::Response>, error_stack::Report<E>>
{
    fn get_error_response(self) -> RouterResult<types::Response> {
        self.change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while receiving response")
            .and_then(|inner| match inner {
                Ok(res) => {
                    logger::error!(response=?res);
                    Err(errors::ApiErrorResponse::InternalServerError)
                        .into_report()
                        .attach_printable(format!(
                            "Expecting error response, received response: {res:?}"
                        ))
                }
                Err(err_res) => Ok(err_res),
            })
    }

    fn get_response(self) -> RouterResult<types::Response> {
        self.change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while receiving response")
            .and_then(|inner| match inner {
                Err(err_res) => {
                    logger::error!(error_response=?err_res);
                    Err(errors::ApiErrorResponse::InternalServerError)
                        .into_report()
                        .attach_printable(format!(
                            "Expecting response, received error response: {err_res:?}"
                        ))
                }
                Ok(res) => Ok(res),
            })
    }
}

/// Convert the amount to its base denomination based on Currency and return String
pub fn to_currency_base_unit(
    amount: i64,
    currency: storage_models::enums::Currency,
) -> Result<String, error_stack::Report<errors::ValidationError>> {
    let amount_f64 = to_currency_base_unit_asf64(amount, currency)?;
    Ok(format!("{amount_f64:.2}"))
}

/// Convert the amount to its base denomination based on Currency and return f64
pub fn to_currency_base_unit_asf64(
    amount: i64,
    currency: storage_models::enums::Currency,
) -> Result<f64, error_stack::Report<errors::ValidationError>> {
    let amount_u32 = u32::try_from(amount).into_report().change_context(
        errors::ValidationError::InvalidValue {
            message: amount.to_string(),
        },
    )?;
    let amount_f64 = f64::from(amount_u32);
    let amount = match currency {
        storage_models::enums::Currency::JPY | storage_models::enums::Currency::KRW => amount_f64,
        storage_models::enums::Currency::BHD
        | storage_models::enums::Currency::JOD
        | storage_models::enums::Currency::KWD
        | storage_models::enums::Currency::OMR => amount_f64 / 1000.00,
        _ => amount_f64 / 100.00,
    };
    Ok(amount)
}

#[inline]
pub fn get_payment_attempt_id(payment_id: impl std::fmt::Display, attempt_count: i16) -> String {
    format!("{payment_id}_{attempt_count}")
}
