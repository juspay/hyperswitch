#[cfg(feature = "olap")]
pub mod connector_onboarding;
pub mod currency;
pub mod custom_serde;
pub mod db_utils;
pub mod ext_traits;
#[cfg(feature = "kv_store")]
pub mod storage_partitioning;
#[cfg(feature = "olap")]
pub mod user;
#[cfg(feature = "olap")]
pub mod user_role;
#[cfg(feature = "olap")]
pub mod verify_connector;

use std::fmt::Debug;

use api_models::{enums, payments, webhooks};
use base64::Engine;
pub use common_utils::{
    crypto,
    ext_traits::{ByteSliceExt, BytesExt, Encode, StringExt, ValueExt},
    fp_utils::when,
    validation::validate_email,
};
use data_models::payments::PaymentIntent;
use error_stack::{IntoReport, ResultExt};
use image::Luma;
use nanoid::nanoid;
use qrcode;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing_futures::Instrument;
use uuid::Uuid;

pub use self::ext_traits::{OptionExt, ValidateCall};
use crate::{
    consts,
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        utils, webhooks as webhooks_core,
    },
    db::StorageInterface,
    logger,
    routes::metrics,
    services,
    types::{
        self,
        domain::{
            self,
            types::{encrypt_optional, AsyncLift},
        },
        storage,
        transformers::ForeignFrom,
    },
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
                /// Formats the error message as a JSON string and writes it to the provided formatter.
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
                /// Returns the status code for a bad request.
        fn status_code(&self) -> StatusCode {
            StatusCode::BAD_REQUEST
        }

                /// This method constructs an error response with the status code and content type
        /// set to application/json, and the response body containing the error message as a string.
        fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
            use actix_web::http::header;

            actix_web::HttpResponseBuilder::new(self.status_code())
                .insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
                .body(self.to_string())
        }
    }

        /// This method takes in a JsonPayloadError and an HttpRequest, and returns an Error.
    /// It creates a custom error using the JsonPayloadError and returns it as an actix_web Error.
    pub fn custom_json_error_handler(err: JsonPayloadError, _req: &HttpRequest) -> Error {
        actix_web::error::Error::from(CustomJsonError { err })
    }
}

#[inline]
/// Generates a unique identifier with the specified length and prefix using the nanoid crate.
/// 
/// # Arguments
/// 
/// * `length` - The length of the generated identifier.
/// * `prefix` - The prefix to be added to the generated identifier.
/// 
pub fn generate_id(length: usize, prefix: &str) -> String {
    format!("{}_{}", prefix, nanoid!(length, &consts::ALPHABETS))
}

#[inline]
/// Generates a new UUID (Universally Unique Identifier) using the version 4 (random) algorithm
///
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub trait ConnectorResponseExt: Sized {
    fn get_response(self) -> RouterResult<types::Response>;
    fn get_error_response(self) -> RouterResult<types::Response>;
        /// This method is used to retrieve and parse the response of an API call into a specified type.
    /// 
    /// # Arguments
    /// 
    /// * `type_name` - The name of the type to parse the response into.
    /// 
    /// # Returns
    /// 
    /// This method returns a `RouterResult` containing the parsed response of type `T`.
    /// 
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
        /// This method is used to get an error response by changing the context to an internal server error, attaching a printable error message, and then handling the response accordingly. If the inner result is Ok, it logs an error and returns an internal server error with a printable message indicating an unexpected response. If the inner result is Err, it returns the error response.
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

        /// This method is used to retrieve a response from the router. It first changes the context to indicate an internal server error, then attaches a printable error message. It then matches the inner result, logging an error if it's an error response and returning an internal server error with a printable message. If it's a success response, it returns the response.
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
/// This method takes a payment_id and an attempt_count and returns a formatted string combining the payment_id and attempt_count.
pub fn get_payment_attempt_id(payment_id: impl std::fmt::Display, attempt_count: i16) -> String {
    format!("{payment_id}_{attempt_count}")
}

#[derive(Debug)]
pub struct QrImage {
    pub data: String,
}

impl QrImage {
        /// Creates a new instance of the struct from the given data string, which represents the content of the QR code. 
    /// Returns a Result with the new instance if successful, or an error report if the QR code creation failed.
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

/// Finds a payment intent based on the provided payment ID type, using the given database and merchant account information.
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

/// Retrieves a PaymentIntent associated with a given refund ID type, merchant account, and connector name from the database.
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

/// Asynchronously finds a payment intent based on the given mandate ID type and merchant account. 
/// 
/// # Arguments
/// 
/// * `db` - A reference to a `StorageInterface` trait object.
/// * `mandate_id_type` - The type of mandate ID used to identify the mandate.
/// * `merchant_account` - A reference to the merchant account for which the payment intent should be found.
/// 
/// # Returns
/// 
/// A `CustomResult` containing the found `PaymentIntent`, or an `ApiErrorResponse` if the payment intent is not found.
/// 
pub async fn find_payment_intent_from_mandate_id_type(
    db: &dyn StorageInterface,
    mandate_id_type: webhooks::MandateIdType,
    merchant_account: &domain::MerchantAccount,
) -> CustomResult<PaymentIntent, errors::ApiErrorResponse> {
    let mandate = match mandate_id_type {
        webhooks::MandateIdType::MandateId(mandate_id) => db
            .find_mandate_by_merchant_id_mandate_id(
                &merchant_account.merchant_id,
                mandate_id.as_str(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?,
        webhooks::MandateIdType::ConnectorMandateId(connector_mandate_id) => db
            .find_mandate_by_merchant_id_connector_mandate_id(
                &merchant_account.merchant_id,
                connector_mandate_id.as_str(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?,
    };
    db.find_payment_intent_by_payment_id_merchant_id(
        &mandate
            .original_payment_id
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable("original_payment_id not present in mandate record")?,
        &merchant_account.merchant_id,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
}

/// Retrieves the profile ID using the object reference ID and merchant account information.
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
                webhooks::ObjectReferenceId::MandateId(mandate_id_type) => {
                    find_payment_intent_from_mandate_id_type(db, mandate_id_type, merchant_account)
                        .await?
                }
            };

            let profile_id = utils::get_profile_id_from_business_details(
                payment_intent.business_country,
                payment_intent.business_label.as_ref(),
                merchant_account,
                payment_intent.profile_id.as_ref(),
                db,
                false,
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
                attempt_status: None,
                connector_transaction_id: None,
            })
        }
    }
}

/// This method takes an HTTP status code as input and returns the type of the status code (1xx, 2xx, 3xx, 4xx, 5xx) as a result. If the input status code is not within the valid range, it returns an error with an internal server error message.
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

/// Adds metrics for the given HTTP status code to the connector HTTP status code metrics. If the provided status code is in the range of 1xx, 2xx, 3xx, 4xx, or 5xx, the corresponding metric count is incremented by 1. If the status code is not within any of these ranges, a log message is generated indicating that the metrics are being skipped due to an invalid status code or no status code being received from the connector.
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

#[async_trait::async_trait]
pub trait CustomerAddress {
    async fn get_address_update(
        &self,
        address_details: api_models::payments::AddressDetails,
        key: &[u8],
        storage_scheme: storage::enums::MerchantStorageScheme,
    ) -> CustomResult<storage::AddressUpdate, common_utils::errors::CryptoError>;

    async fn get_domain_address(
        &self,
        address_details: api_models::payments::AddressDetails,
        merchant_id: &str,
        customer_id: &str,
        key: &[u8],
        storage_scheme: storage::enums::MerchantStorageScheme,
    ) -> CustomResult<domain::Address, common_utils::errors::CryptoError>;
}

#[async_trait::async_trait]
impl CustomerAddress for api_models::customers::CustomerRequest {
        /// Asynchronously updates the address details with encryption and returns a CustomResult containing the updated address details or a CryptoError if encryption fails.
    async fn get_address_update(
        &self,
        address_details: api_models::payments::AddressDetails,
        key: &[u8],
        storage_scheme: storage::enums::MerchantStorageScheme,
    ) -> CustomResult<storage::AddressUpdate, common_utils::errors::CryptoError> {
        async {
            Ok(storage::AddressUpdate::Update {
                city: address_details.city,
                country: address_details.country,
                line1: address_details
                    .line1
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                line2: address_details
                    .line2
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                line3: address_details
                    .line3
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                zip: address_details
                    .zip
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                state: address_details
                    .state
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                first_name: address_details
                    .first_name
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                last_name: address_details
                    .last_name
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                phone_number: self
                    .phone
                    .clone()
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                country_code: self.phone_country_code.clone(),
                updated_by: storage_scheme.to_string(),
            })
        }
        .await
    }

        /// Asynchronously retrieves the domain address of a customer using the provided address details, merchant ID, customer ID, encryption key, and storage scheme. The address details are encrypted before being stored and the resulting domain address is returned as a CustomResult.
    async fn get_domain_address(
        &self,
        address_details: api_models::payments::AddressDetails,
        merchant_id: &str,
        customer_id: &str,
        key: &[u8],
        storage_scheme: storage::enums::MerchantStorageScheme,
    ) -> CustomResult<domain::Address, common_utils::errors::CryptoError> {
        async {
            Ok(domain::Address {
                id: None,
                city: address_details.city,
                country: address_details.country,
                line1: address_details
                    .line1
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                line2: address_details
                    .line2
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                line3: address_details
                    .line3
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                zip: address_details
                    .zip
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                state: address_details
                    .state
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                first_name: address_details
                    .first_name
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                last_name: address_details
                    .last_name
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                phone_number: self
                    .phone
                    .clone()
                    .async_lift(|inner| encrypt_optional(inner, key))
                    .await?,
                country_code: self.phone_country_code.clone(),
                customer_id: Some(customer_id.to_string()),
                merchant_id: merchant_id.to_string(),
                address_id: generate_id(consts::ID_LENGTH, "add"),
                payment_id: None,
                created_at: common_utils::date_time::now(),
                modified_at: common_utils::date_time::now(),
                updated_by: storage_scheme.to_string(),
            })
        }
        .await
    }
}

/// Adds metrics for the Apple Pay flow based on the provided Apple Pay flow type, connector, and merchant ID.
pub fn add_apple_pay_flow_metrics(
    apple_pay_flow: &Option<enums::ApplePayFlow>,
    connector: Option<String>,
    merchant_id: String,
) {
    if let Some(flow) = apple_pay_flow {
        match flow {
            enums::ApplePayFlow::Simplified => metrics::APPLE_PAY_SIMPLIFIED_FLOW.add(
                &metrics::CONTEXT,
                1,
                &[
                    metrics::request::add_attributes(
                        "connector",
                        connector.to_owned().unwrap_or("null".to_string()),
                    ),
                    metrics::request::add_attributes("merchant_id", merchant_id.to_owned()),
                ],
            ),
            enums::ApplePayFlow::Manual => metrics::APPLE_PAY_MANUAL_FLOW.add(
                &metrics::CONTEXT,
                1,
                &[
                    metrics::request::add_attributes(
                        "connector",
                        connector.to_owned().unwrap_or("null".to_string()),
                    ),
                    metrics::request::add_attributes("merchant_id", merchant_id.to_owned()),
                ],
            ),
        }
    }
}

/// This method adds metrics for Apple Pay payment status based on the payment attempt status, Apple Pay flow, connector, and merchant ID.
pub fn add_apple_pay_payment_status_metrics(
    payment_attempt_status: enums::AttemptStatus,
    apple_pay_flow: Option<enums::ApplePayFlow>,
    connector: Option<String>,
    merchant_id: String,
) {
    if payment_attempt_status == enums::AttemptStatus::Charged {
        if let Some(flow) = apple_pay_flow {
            match flow {
                enums::ApplePayFlow::Simplified => {
                    metrics::APPLE_PAY_SIMPLIFIED_FLOW_SUCCESSFUL_PAYMENT.add(
                        &metrics::CONTEXT,
                        1,
                        &[
                            metrics::request::add_attributes(
                                "connector",
                                connector.to_owned().unwrap_or("null".to_string()),
                            ),
                            metrics::request::add_attributes("merchant_id", merchant_id.to_owned()),
                        ],
                    )
                }
                enums::ApplePayFlow::Manual => metrics::APPLE_PAY_MANUAL_FLOW_SUCCESSFUL_PAYMENT
                    .add(
                        &metrics::CONTEXT,
                        1,
                        &[
                            metrics::request::add_attributes(
                                "connector",
                                connector.to_owned().unwrap_or("null".to_string()),
                            ),
                            metrics::request::add_attributes("merchant_id", merchant_id.to_owned()),
                        ],
                    ),
            }
        }
    } else if payment_attempt_status == enums::AttemptStatus::Failure {
        if let Some(flow) = apple_pay_flow {
            match flow {
                enums::ApplePayFlow::Simplified => {
                    metrics::APPLE_PAY_SIMPLIFIED_FLOW_FAILED_PAYMENT.add(
                        &metrics::CONTEXT,
                        1,
                        &[
                            metrics::request::add_attributes(
                                "connector",
                                connector.to_owned().unwrap_or("null".to_string()),
                            ),
                            metrics::request::add_attributes("merchant_id", merchant_id.to_owned()),
                        ],
                    )
                }
                enums::ApplePayFlow::Manual => metrics::APPLE_PAY_MANUAL_FLOW_FAILED_PAYMENT.add(
                    &metrics::CONTEXT,
                    1,
                    &[
                        metrics::request::add_attributes(
                            "connector",
                            connector.to_owned().unwrap_or("null".to_string()),
                        ),
                        metrics::request::add_attributes("merchant_id", merchant_id.to_owned()),
                    ],
                ),
            }
        }
    }
}

/// Trigger a payments webhook based on the status of a payment intent, and send the appropriate outgoing webhook if the status is one of Succeeded, Failed, or PartiallyCaptured.
pub async fn trigger_payments_webhook<F, Req, Op>(
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    payment_data: crate::core::payments::PaymentData<F>,
    req: Option<Req>,
    customer: Option<domain::Customer>,
    state: &crate::routes::AppState,
    operation: Op,
) -> RouterResult<()>
where
    F: Send + Clone + Sync,
    Op: Debug,
{
    let status = payment_data.payment_intent.status;
    let payment_id = payment_data.payment_intent.payment_id.clone();
    let captures = payment_data
        .multiple_capture_data
        .clone()
        .map(|multiple_capture_data| {
            multiple_capture_data
                .get_all_captures()
                .into_iter()
                .cloned()
                .collect()
        });

    if matches!(
        status,
        enums::IntentStatus::Succeeded
            | enums::IntentStatus::Failed
            | enums::IntentStatus::PartiallyCaptured
    ) {
        let payments_response = crate::core::payments::transformers::payments_to_payments_response(
            req,
            payment_data,
            captures,
            customer,
            services::AuthFlow::Merchant,
            &state.conf.server,
            &operation,
            &state.conf.connector_request_reference_id_config,
            None,
            None,
            None,
        )?;

        let event_type = ForeignFrom::foreign_from(status);

        if let services::ApplicationResponse::JsonWithHeaders((payments_response_json, _)) =
            payments_response
        {
            let m_state = state.clone();
            // This spawns this futures in a background thread, the exception inside this future won't affect
            // the current thread and the lifecycle of spawn thread is not handled by runtime.
            // So when server shutdown won't wait for this thread's completion.

            if let Some(event_type) = event_type {
                tokio::spawn(
                    async move {
                        Box::pin(
                            webhooks_core::create_event_and_trigger_appropriate_outgoing_webhook(
                                m_state,
                                merchant_account,
                                business_profile,
                                event_type,
                                diesel_models::enums::EventClass::Payments,
                                None,
                                payment_id,
                                diesel_models::enums::EventObjectType::PaymentDetails,
                                webhooks::OutgoingWebhookContent::PaymentDetails(
                                    payments_response_json,
                                ),
                            ),
                        )
                        .await
                    }
                    .in_current_span(),
                );
            } else {
                logger::warn!(
                    "Outgoing webhook not sent because of missing event type status mapping"
                );
            }
        }
    }

    Ok(())
}

type Handle<T> = tokio::task::JoinHandle<RouterResult<T>>;

/// Asynchronously flattens the result of a `Handle` into a `RouterResult`.
/// If the `Handle` resolves to `Ok(Ok(t))`, returns `Ok(t)`.
/// If the `Handle` resolves to `Ok(Err(err))`, returns `Err(err)`.
/// If the `Handle` resolves to `Err(err)`, transforms the error into a report, changes the context to `InternalServerError`, and attaches a printable message "Join Error".
pub async fn flatten_join_error<T>(handle: Handle<T>) -> RouterResult<T> {
    match handle.await {
        Ok(Ok(t)) => Ok(t),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err)
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Join Error"),
    }
}

#[cfg(test)]
mod tests {
    use crate::utils;
    #[test]
        /// This method tests the functionality of creating a QR image data source URL from the given data.
    fn test_image_data_source_url() {
        let qr_image_data_source_url = utils::QrImage::new_from_data("Hyperswitch".to_string());
        assert!(qr_image_data_source_url.is_ok());
    }
}
