pub mod custom_serde;
pub mod db_utils;
pub mod ext_traits;
#[cfg(all(feature = "olap", feature = "kms"))]
pub mod verification;

#[cfg(feature = "kv_store")]
pub mod storage_partitioning;

use api_models::{payments, webhooks};
use base64::Engine;
pub use common_utils::{
    crypto,
    ext_traits::{ByteSliceExt, BytesExt, Encode, StringExt, ValueExt},
    fp_utils::when,
    validation::validate_email,
};
use data_models::payments::payment_intent::PaymentIntent;
use error_stack::{IntoReport, ResultExt};
use image::Luma;
use nanoid::nanoid;
use qrcode;
use serde::de::DeserializeOwned;
use serde_json::Value;
use uuid::Uuid;

pub use self::ext_traits::{OptionExt, ValidateCall};
use crate::{
    consts,
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        utils,
    },
    db::StorageInterface,
    logger,
    routes::metrics,
    types::{self, domain},
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

#[inline]
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
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

#[inline]
pub fn get_payment_attempt_id(payment_id: impl std::fmt::Display, attempt_count: i16) -> String {
    format!("{payment_id}_{attempt_count}")
}

#[derive(Debug)]
pub struct QrImage {
    pub data: String,
}

impl QrImage {
    pub fn new_from_data(
        data: String,
    ) -> Result<Self, error_stack::Report<common_utils::errors::QrCodeError>> {
        let qr_code = qrcode::QrCode::new(data.as_bytes())
            .into_report()
            .change_context(common_utils::errors::QrCodeError::FailedToCreateQrCode)?;

        // Renders the QR code into an image.
        let qrcode_image_buffer = qr_code.render::<Luma<u8>>().build();
        let qrcode_dynamic_image = image::DynamicImage::ImageLuma8(qrcode_image_buffer);

        let mut image_bytes = Vec::new();

        // Encodes qrcode_dynamic_image and write it to image_bytes
        let _ = qrcode_dynamic_image.write_to(&mut image_bytes, image::ImageOutputFormat::Png);

        let image_data_source = format!(
            "{},{}",
            consts::QR_IMAGE_DATA_SOURCE_STRING,
            consts::BASE64_ENGINE.encode(image_bytes)
        );
        Ok(Self {
            data: image_data_source,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::utils;
    #[test]
    fn test_image_data_source_url() {
        let qr_image_data_source_url = utils::QrImage::new_from_data("Hyperswitch".to_string());
        assert!(qr_image_data_source_url.is_ok());
    }
}

pub async fn find_payment_intent_from_payment_id_type(
    db: &dyn StorageInterface,
    payment_id_type: payments::PaymentIdType,
    merchant_account: &domain::MerchantAccount,
) -> CustomResult<PaymentIntent, errors::ApiErrorResponse> {
    match payment_id_type {
        payments::PaymentIdType::PaymentIntentId(payment_id) => db
            .find_payment_intent_by_payment_id_merchant_id(
                &payment_id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound),
        payments::PaymentIdType::ConnectorTransactionId(connector_transaction_id) => {
            let attempt = db
                .find_payment_attempt_by_merchant_id_connector_txn_id(
                    &merchant_account.merchant_id,
                    &connector_transaction_id,
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            db.find_payment_intent_by_payment_id_merchant_id(
                &attempt.payment_id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
        }
        payments::PaymentIdType::PaymentAttemptId(attempt_id) => {
            let attempt = db
                .find_payment_attempt_by_attempt_id_merchant_id(
                    &attempt_id,
                    &merchant_account.merchant_id,
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            db.find_payment_intent_by_payment_id_merchant_id(
                &attempt.payment_id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
        }
        payments::PaymentIdType::PreprocessingId(_) => {
            Err(errors::ApiErrorResponse::PaymentNotFound)?
        }
    }
}

pub async fn find_payment_intent_from_refund_id_type(
    db: &dyn StorageInterface,
    refund_id_type: webhooks::RefundIdType,
    merchant_account: &domain::MerchantAccount,
    connector_name: &str,
) -> CustomResult<PaymentIntent, errors::ApiErrorResponse> {
    let refund = match refund_id_type {
        webhooks::RefundIdType::RefundId(id) => db
            .find_refund_by_merchant_id_refund_id(
                &merchant_account.merchant_id,
                &id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?,
        webhooks::RefundIdType::ConnectorRefundId(id) => db
            .find_refund_by_merchant_id_connector_refund_id_connector(
                &merchant_account.merchant_id,
                &id,
                connector_name,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?,
    };
    let attempt = db
        .find_payment_attempt_by_attempt_id_merchant_id(
            &refund.attempt_id,
            &merchant_account.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
    db.find_payment_intent_by_payment_id_merchant_id(
        &attempt.payment_id,
        &merchant_account.merchant_id,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
}

pub async fn get_profile_id_using_object_reference_id(
    db: &dyn StorageInterface,
    object_reference_id: webhooks::ObjectReferenceId,
    merchant_account: &domain::MerchantAccount,
    connector_name: &str,
) -> CustomResult<String, errors::ApiErrorResponse> {
    match merchant_account.default_profile.as_ref() {
        Some(profile_id) => Ok(profile_id.clone()),
        _ => {
            let payment_intent = match object_reference_id {
                webhooks::ObjectReferenceId::PaymentId(payment_id_type) => {
                    find_payment_intent_from_payment_id_type(db, payment_id_type, merchant_account)
                        .await?
                }
                webhooks::ObjectReferenceId::RefundId(refund_id_type) => {
                    find_payment_intent_from_refund_id_type(
                        db,
                        refund_id_type,
                        merchant_account,
                        connector_name,
                    )
                    .await?
                }
            };

            let profile_id = utils::get_profile_id_from_business_details(
                payment_intent.business_country,
                payment_intent.business_label.as_ref(),
                merchant_account,
                payment_intent.profile_id.as_ref(),
                db,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("profile_id is not set in payment_intent")?;

            Ok(profile_id)
        }
    }
}

// validate json format for the error
pub fn handle_json_response_deserialization_failure(
    res: types::Response,
    connector: String,
) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
    metrics::RESPONSE_DESERIALIZATION_FAILURE.add(
        &metrics::CONTEXT,
        1,
        &[metrics::request::add_attributes("connector", connector)],
    );

    let response_data = String::from_utf8(res.response.to_vec())
        .into_report()
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

    // check for whether the response is in json format
    match serde_json::from_str::<Value>(&response_data) {
        // in case of unexpected response but in json format
        Ok(_) => Err(errors::ConnectorError::ResponseDeserializationFailed)?,
        // in case of unexpected response but in html or string format
        Err(error_msg) => {
            logger::error!(deserialization_error=?error_msg);
            logger::error!("UNEXPECTED RESPONSE FROM CONNECTOR: {}", response_data);
            Ok(types::ErrorResponse {
                status_code: res.status_code,
                code: consts::NO_ERROR_CODE.to_string(),
                message: consts::UNSUPPORTED_ERROR_MESSAGE.to_string(),
                reason: Some(response_data),
            })
        }
    }
}

pub fn get_http_status_code_type(
    status_code: u16,
) -> CustomResult<String, errors::ApiErrorResponse> {
    let status_code_type = match status_code {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => Err(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable("Invalid http status code")?,
    };
    Ok(status_code_type.to_string())
}

pub fn add_connector_http_status_code_metrics(option_status_code: Option<u16>) {
    if let Some(status_code) = option_status_code {
        let status_code_type = get_http_status_code_type(status_code).ok();
        match status_code_type.as_deref() {
            Some("1xx") => {
                metrics::CONNECTOR_HTTP_STATUS_CODE_1XX_COUNT.add(&metrics::CONTEXT, 1, &[])
            }
            Some("2xx") => {
                metrics::CONNECTOR_HTTP_STATUS_CODE_2XX_COUNT.add(&metrics::CONTEXT, 1, &[])
            }
            Some("3xx") => {
                metrics::CONNECTOR_HTTP_STATUS_CODE_3XX_COUNT.add(&metrics::CONTEXT, 1, &[])
            }
            Some("4xx") => {
                metrics::CONNECTOR_HTTP_STATUS_CODE_4XX_COUNT.add(&metrics::CONTEXT, 1, &[])
            }
            Some("5xx") => {
                metrics::CONNECTOR_HTTP_STATUS_CODE_5XX_COUNT.add(&metrics::CONTEXT, 1, &[])
            }
            _ => logger::info!("Skip metrics as invalid http status code received from connector"),
        };
    } else {
        logger::info!("Skip metrics as no http status code received from connector")
    }
}
