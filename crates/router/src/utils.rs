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
use error_stack::ResultExt;
use image::Luma;
use masking::ExposeInterface;
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
        self.map_err(|error| error.change_context(errors::ApiErrorResponse::InternalServerError))
            .attach_printable("Error while receiving response")
            .and_then(|inner| match inner {
                Ok(res) => {
                    logger::error!(response=?res);
                    Err(errors::ApiErrorResponse::InternalServerError).attach_printable(format!(
                        "Expecting error response, received response: {res:?}"
                    ))
                }
                Err(err_res) => Ok(err_res),
            })
    }

    fn get_response(self) -> RouterResult<types::Response> {
        self.map_err(|error| error.change_context(errors::ApiErrorResponse::InternalServerError))
            .attach_printable("Error while receiving response")
            .and_then(|inner| match inner {
                Err(err_res) => {
                    logger::error!(error_response=?err_res);
                    Err(errors::ApiErrorResponse::InternalServerError).attach_printable(format!(
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
            .change_context(common_utils::errors::QrCodeError::FailedToCreateQrCode)?;

        // Renders the QR code into an image.
        let qrcode_image_buffer = qr_code.render::<Luma<u8>>().build();
        let qrcode_dynamic_image = image::DynamicImage::ImageLuma8(qrcode_image_buffer);

        let mut image_bytes = std::io::BufWriter::new(std::io::Cursor::new(Vec::new()));

        // Encodes qrcode_dynamic_image and write it to image_bytes
        let _ = qrcode_dynamic_image.write_to(&mut image_bytes, image::ImageFormat::Png);

        let image_data_source = format!(
            "{},{}",
            consts::QR_IMAGE_DATA_SOURCE_STRING,
            consts::BASE64_ENGINE.encode(image_bytes.get_ref().get_ref())
        );
        Ok(Self {
            data: image_data_source,
        })
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
            .attach_printable("original_payment_id not present in mandate record")?,
        &merchant_account.merchant_id,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
}

pub async fn find_mca_from_authentication_id_type(
    db: &dyn StorageInterface,
    authentication_id_type: webhooks::AuthenticationIdType,
    merchant_account: &domain::MerchantAccount,
    connector_name: &str,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<domain::MerchantConnectorAccount, errors::ApiErrorResponse> {
    let authentication = match authentication_id_type {
        webhooks::AuthenticationIdType::AuthenticationId(authentication_id) => db
            .find_authentication_by_merchant_id_authentication_id(
                merchant_account.merchant_id.clone(),
                authentication_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?,
        webhooks::AuthenticationIdType::ConnectorAuthenticationId(connector_authentication_id) => {
            db.find_authentication_by_merchant_id_connector_authentication_id(
                merchant_account.merchant_id.clone(),
                connector_authentication_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?
        }
    };
    let profile_id = authentication.profile_id;
    db.find_merchant_connector_account_by_profile_id_connector_name(
        &profile_id,
        connector_name,
        key_store,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
        id: format!("profile_id {profile_id} and connector_name {connector_name}"),
    })
}

pub async fn get_mca_from_payment_intent(
    db: &dyn StorageInterface,
    merchant_account: &domain::MerchantAccount,
    payment_intent: PaymentIntent,
    key_store: &domain::MerchantKeyStore,
    connector_name: &str,
) -> CustomResult<domain::MerchantConnectorAccount, errors::ApiErrorResponse> {
    let payment_attempt = db
        .find_payment_attempt_by_attempt_id_merchant_id(
            &payment_intent.active_attempt.get_id(),
            &merchant_account.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    match payment_attempt.merchant_connector_id {
        Some(merchant_connector_id) => db
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                &merchant_account.merchant_id,
                &merchant_connector_id,
                key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: merchant_connector_id,
            }),
        None => {
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

            db.find_merchant_connector_account_by_profile_id_connector_name(
                &profile_id,
                connector_name,
                key_store,
            )
            .await
            .to_not_found_response(
                errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                    id: format!("profile_id {profile_id} and connector_name {connector_name}"),
                },
            )
        }
    }
}

pub async fn get_mca_from_object_reference_id(
    db: &dyn StorageInterface,
    object_reference_id: webhooks::ObjectReferenceId,
    merchant_account: &domain::MerchantAccount,
    connector_name: &str,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<domain::MerchantConnectorAccount, errors::ApiErrorResponse> {
    match merchant_account.default_profile.as_ref() {
        Some(profile_id) => db
            .find_merchant_connector_account_by_profile_id_connector_name(
                profile_id,
                connector_name,
                key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: format!("profile_id {profile_id} and connector_name {connector_name}"),
            }),
        _ => match object_reference_id {
            webhooks::ObjectReferenceId::PaymentId(payment_id_type) => {
                get_mca_from_payment_intent(
                    db,
                    merchant_account,
                    find_payment_intent_from_payment_id_type(db, payment_id_type, merchant_account)
                        .await?,
                    key_store,
                    connector_name,
                )
                .await
            }
            webhooks::ObjectReferenceId::RefundId(refund_id_type) => {
                get_mca_from_payment_intent(
                    db,
                    merchant_account,
                    find_payment_intent_from_refund_id_type(
                        db,
                        refund_id_type,
                        merchant_account,
                        connector_name,
                    )
                    .await?,
                    key_store,
                    connector_name,
                )
                .await
            }
            webhooks::ObjectReferenceId::MandateId(mandate_id_type) => {
                get_mca_from_payment_intent(
                    db,
                    merchant_account,
                    find_payment_intent_from_mandate_id_type(db, mandate_id_type, merchant_account)
                        .await?,
                    key_store,
                    connector_name,
                )
                .await
            }
            webhooks::ObjectReferenceId::ExternalAuthenticationID(authentication_id_type) => {
                find_mca_from_authentication_id_type(
                    db,
                    authentication_id_type,
                    merchant_account,
                    connector_name,
                    key_store,
                )
                .await
            }
        },
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
                email: self
                    .email
                    .as_ref()
                    .cloned()
                    .async_lift(|inner| encrypt_optional(inner.map(|inner| inner.expose()), key))
                    .await?,
            })
        }
        .await
    }

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
                email: self
                    .email
                    .as_ref()
                    .cloned()
                    .async_lift(|inner| encrypt_optional(inner.map(|inner| inner.expose()), key))
                    .await?,
            })
        }
        .await
    }
}

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

#[allow(clippy::too_many_arguments)]
pub async fn trigger_payments_webhook<F, Op>(
    merchant_account: domain::MerchantAccount,
    business_profile: diesel_models::business_profile::BusinessProfile,
    key_store: &domain::MerchantKeyStore,
    payment_data: crate::core::payments::PaymentData<F>,
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
            let cloned_state = state.clone();
            let cloned_key_store = key_store.clone();
            // This spawns this futures in a background thread, the exception inside this future won't affect
            // the current thread and the lifecycle of spawn thread is not handled by runtime.
            // So when server shutdown won't wait for this thread's completion.

            if let Some(event_type) = event_type {
                tokio::spawn(
                    async move {
                        let primary_object_created_at = payments_response_json.created;
                        Box::pin(webhooks_core::create_event_and_trigger_outgoing_webhook(
                            cloned_state,
                            merchant_account,
                            business_profile,
                            &cloned_key_store,
                            event_type,
                            diesel_models::enums::EventClass::Payments,
                            payment_id,
                            diesel_models::enums::EventObjectType::PaymentDetails,
                            webhooks::OutgoingWebhookContent::PaymentDetails(
                                payments_response_json,
                            ),
                            primary_object_created_at,
                        ))
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

pub async fn flatten_join_error<T>(handle: Handle<T>) -> RouterResult<T> {
    match handle.await {
        Ok(Ok(t)) => Ok(t),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Join Error"),
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
