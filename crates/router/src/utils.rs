#[cfg(feature = "olap")]
pub mod connector_onboarding;
pub mod currency;
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

use api_models::{
    enums,
    payments::{self},
    webhooks,
};
use common_utils::types::keymanager::KeyManagerState;
pub use common_utils::{
    crypto::{self, Encryptable},
    ext_traits::{ByteSliceExt, BytesExt, Encode, StringExt, ValueExt},
    fp_utils::when,
    id_type, pii,
    validation::validate_email,
};
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use common_utils::{
    type_name,
    types::keymanager::{Identifier, ToEncryptable},
};
use error_stack::ResultExt;
pub use hyperswitch_connectors::utils::QrImage;
use hyperswitch_domain_models::payments::PaymentIntent;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use hyperswitch_domain_models::type_encryption::{crypto_operation, CryptoOperation};
use masking::{ExposeInterface, SwitchStrategy};
use nanoid::nanoid;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing_futures::Instrument;
use uuid::Uuid;

pub use self::ext_traits::{OptionExt, ValidateCall};
#[cfg(feature = "v1")]
use crate::core::webhooks as webhooks_core;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use crate::types::storage;
use crate::{
    consts,
    core::{
        authentication::types::ExternalThreeDSConnectorMetadata,
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payments as payments_core,
    },
    headers::ACCEPT_LANGUAGE,
    logger,
    routes::{metrics, SessionState},
    services::{self, authentication::get_header_value_by_key},
    types::{
        self, domain,
        transformers::{ForeignFrom, ForeignInto},
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
                    "error": {
                        "error_type": "invalid_request",
                        "message": self.err.to_string(),
                        "code": "IR_06",
                    }
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
        Error::from(CustomJsonError { err })
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
pub fn get_payout_attempt_id(payout_id: impl std::fmt::Display, attempt_count: i16) -> String {
    format!("{payout_id}_{attempt_count}")
}

#[cfg(feature = "v1")]
pub async fn find_payment_intent_from_payment_id_type(
    state: &SessionState,
    payment_id_type: payments::PaymentIdType,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<PaymentIntent, errors::ApiErrorResponse> {
    let key_manager_state: KeyManagerState = state.into();
    let db = &*state.store;
    match payment_id_type {
        payments::PaymentIdType::PaymentIntentId(payment_id) => db
            .find_payment_intent_by_payment_id_merchant_id(
                &key_manager_state,
                &payment_id,
                merchant_account.get_id(),
                key_store,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound),
        payments::PaymentIdType::ConnectorTransactionId(connector_transaction_id) => {
            let attempt = db
                .find_payment_attempt_by_merchant_id_connector_txn_id(
                    merchant_account.get_id(),
                    &connector_transaction_id,
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            db.find_payment_intent_by_payment_id_merchant_id(
                &key_manager_state,
                &attempt.payment_id,
                merchant_account.get_id(),
                key_store,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
        }
        payments::PaymentIdType::PaymentAttemptId(attempt_id) => {
            let attempt = db
                .find_payment_attempt_by_attempt_id_merchant_id(
                    &attempt_id,
                    merchant_account.get_id(),
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
            db.find_payment_intent_by_payment_id_merchant_id(
                &key_manager_state,
                &attempt.payment_id,
                merchant_account.get_id(),
                key_store,
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

#[cfg(feature = "v1")]
pub async fn find_payment_intent_from_refund_id_type(
    state: &SessionState,
    refund_id_type: webhooks::RefundIdType,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector_name: &str,
) -> CustomResult<PaymentIntent, errors::ApiErrorResponse> {
    let db = &*state.store;
    let refund = match refund_id_type {
        webhooks::RefundIdType::RefundId(id) => db
            .find_refund_by_merchant_id_refund_id(
                merchant_account.get_id(),
                &id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?,
        webhooks::RefundIdType::ConnectorRefundId(id) => db
            .find_refund_by_merchant_id_connector_refund_id_connector(
                merchant_account.get_id(),
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
            merchant_account.get_id(),
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
    db.find_payment_intent_by_payment_id_merchant_id(
        &state.into(),
        &attempt.payment_id,
        merchant_account.get_id(),
        key_store,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
}

#[cfg(feature = "v1")]
pub async fn find_payment_intent_from_mandate_id_type(
    state: &SessionState,
    mandate_id_type: webhooks::MandateIdType,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<PaymentIntent, errors::ApiErrorResponse> {
    let db = &*state.store;
    let mandate = match mandate_id_type {
        webhooks::MandateIdType::MandateId(mandate_id) => db
            .find_mandate_by_merchant_id_mandate_id(
                merchant_account.get_id(),
                mandate_id.as_str(),
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?,
        webhooks::MandateIdType::ConnectorMandateId(connector_mandate_id) => db
            .find_mandate_by_merchant_id_connector_mandate_id(
                merchant_account.get_id(),
                connector_mandate_id.as_str(),
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MandateNotFound)?,
    };
    db.find_payment_intent_by_payment_id_merchant_id(
        &state.into(),
        &mandate
            .original_payment_id
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("original_payment_id not present in mandate record")?,
        merchant_account.get_id(),
        key_store,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
}

#[cfg(feature = "v1")]
pub async fn find_mca_from_authentication_id_type(
    state: &SessionState,
    authentication_id_type: webhooks::AuthenticationIdType,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<domain::MerchantConnectorAccount, errors::ApiErrorResponse> {
    let db = &*state.store;
    let authentication = match authentication_id_type {
        webhooks::AuthenticationIdType::AuthenticationId(authentication_id) => db
            .find_authentication_by_merchant_id_authentication_id(
                merchant_account.get_id(),
                authentication_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?,
        webhooks::AuthenticationIdType::ConnectorAuthenticationId(connector_authentication_id) => {
            db.find_authentication_by_merchant_id_connector_authentication_id(
                merchant_account.get_id().clone(),
                connector_authentication_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?
        }
    };
    #[cfg(feature = "v1")]
    {
        db.find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &state.into(),
            merchant_account.get_id(),
            &authentication.merchant_connector_id,
            key_store,
        )
        .await
        .to_not_found_response(
            errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: authentication
                    .merchant_connector_id
                    .get_string_repr()
                    .to_string(),
            },
        )
    }
    #[cfg(feature = "v2")]
    //get mca using id
    {
        let _ = key_store;
        let _ = authentication;
        todo!()
    }
}

#[cfg(feature = "v1")]
pub async fn get_mca_from_payment_intent(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    payment_intent: PaymentIntent,
    key_store: &domain::MerchantKeyStore,
    connector_name: &str,
) -> CustomResult<domain::MerchantConnectorAccount, errors::ApiErrorResponse> {
    let db = &*state.store;
    let key_manager_state: &KeyManagerState = &state.into();

    #[cfg(feature = "v1")]
    let payment_attempt = db
        .find_payment_attempt_by_attempt_id_merchant_id(
            &payment_intent.active_attempt.get_id(),
            merchant_account.get_id(),
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    #[cfg(feature = "v2")]
    let payment_attempt = db
        .find_payment_attempt_by_attempt_id_merchant_id(
            key_manager_state,
            key_store,
            &payment_intent.active_attempt.get_id(),
            merchant_account.get_id(),
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    match payment_attempt.merchant_connector_id {
        Some(merchant_connector_id) => {
            #[cfg(feature = "v1")]
            {
                db.find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    key_manager_state,
                    merchant_account.get_id(),
                    &merchant_connector_id,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: merchant_connector_id.get_string_repr().to_string(),
                    },
                )
            }
            #[cfg(feature = "v2")]
            {
                //get mca using id
                let _id = merchant_connector_id;
                let _ = key_store;
                let _ = key_manager_state;
                let _ = connector_name;
                todo!()
            }
        }
        None => {
            let profile_id = payment_intent
                .profile_id
                .as_ref()
                .get_required_value("profile_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("profile_id is not set in payment_intent")?
                .clone();

            #[cfg(feature = "v1")]
            {
                db.find_merchant_connector_account_by_profile_id_connector_name(
                    key_manager_state,
                    &profile_id,
                    connector_name,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: format!(
                            "profile_id {} and connector_name {connector_name}",
                            profile_id.get_string_repr()
                        ),
                    },
                )
            }
            #[cfg(feature = "v2")]
            {
                //get mca using id
                let _ = profile_id;
                todo!()
            }
        }
    }
}

#[cfg(feature = "payouts")]
pub async fn get_mca_from_payout_attempt(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    payout_id_type: webhooks::PayoutIdType,
    connector_name: &str,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<domain::MerchantConnectorAccount, errors::ApiErrorResponse> {
    let db = &*state.store;
    let payout = match payout_id_type {
        webhooks::PayoutIdType::PayoutAttemptId(payout_attempt_id) => db
            .find_payout_attempt_by_merchant_id_payout_attempt_id(
                merchant_account.get_id(),
                &payout_attempt_id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?,
        webhooks::PayoutIdType::ConnectorPayoutId(connector_payout_id) => db
            .find_payout_attempt_by_merchant_id_connector_payout_id(
                merchant_account.get_id(),
                &connector_payout_id,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PayoutNotFound)?,
    };
    let key_manager_state: &KeyManagerState = &state.into();
    match payout.merchant_connector_id {
        Some(merchant_connector_id) => {
            #[cfg(feature = "v1")]
            {
                db.find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    key_manager_state,
                    merchant_account.get_id(),
                    &merchant_connector_id,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: merchant_connector_id.get_string_repr().to_string(),
                    },
                )
            }
            #[cfg(feature = "v2")]
            {
                //get mca using id
                let _id = merchant_connector_id;
                let _ = key_store;
                let _ = connector_name;
                let _ = key_manager_state;
                todo!()
            }
        }
        None => {
            #[cfg(feature = "v1")]
            {
                db.find_merchant_connector_account_by_profile_id_connector_name(
                    key_manager_state,
                    &payout.profile_id,
                    connector_name,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: format!(
                            "profile_id {} and connector_name {}",
                            payout.profile_id.get_string_repr(),
                            connector_name
                        ),
                    },
                )
            }
            #[cfg(feature = "v2")]
            {
                todo!()
            }
        }
    }
}

#[cfg(feature = "v1")]
pub async fn get_mca_from_object_reference_id(
    state: &SessionState,
    object_reference_id: webhooks::ObjectReferenceId,
    merchant_account: &domain::MerchantAccount,
    connector_name: &str,
    key_store: &domain::MerchantKeyStore,
) -> CustomResult<domain::MerchantConnectorAccount, errors::ApiErrorResponse> {
    let db = &*state.store;

    #[cfg(feature = "v1")]
    let default_profile_id = merchant_account.default_profile.as_ref();

    #[cfg(feature = "v2")]
    let default_profile_id = Option::<&String>::None;

    match default_profile_id {
        Some(profile_id) => {
            #[cfg(feature = "v1")]
            {
                db.find_merchant_connector_account_by_profile_id_connector_name(
                    &state.into(),
                    profile_id,
                    connector_name,
                    key_store,
                )
                .await
                .to_not_found_response(
                    errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: format!(
                            "profile_id {} and connector_name {connector_name}",
                            profile_id.get_string_repr()
                        ),
                    },
                )
            }
            #[cfg(feature = "v2")]
            {
                let _db = db;
                let _profile_id = profile_id;
                todo!()
            }
        }
        _ => match object_reference_id {
            webhooks::ObjectReferenceId::PaymentId(payment_id_type) => {
                get_mca_from_payment_intent(
                    state,
                    merchant_account,
                    find_payment_intent_from_payment_id_type(
                        state,
                        payment_id_type,
                        merchant_account,
                        key_store,
                    )
                    .await?,
                    key_store,
                    connector_name,
                )
                .await
            }
            webhooks::ObjectReferenceId::RefundId(refund_id_type) => {
                get_mca_from_payment_intent(
                    state,
                    merchant_account,
                    find_payment_intent_from_refund_id_type(
                        state,
                        refund_id_type,
                        merchant_account,
                        key_store,
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
                    state,
                    merchant_account,
                    find_payment_intent_from_mandate_id_type(
                        state,
                        mandate_id_type,
                        merchant_account,
                        key_store,
                    )
                    .await?,
                    key_store,
                    connector_name,
                )
                .await
            }
            webhooks::ObjectReferenceId::ExternalAuthenticationID(authentication_id_type) => {
                find_mca_from_authentication_id_type(
                    state,
                    authentication_id_type,
                    merchant_account,
                    key_store,
                )
                .await
            }
            #[cfg(feature = "payouts")]
            webhooks::ObjectReferenceId::PayoutId(payout_id_type) => {
                get_mca_from_payout_attempt(
                    state,
                    merchant_account,
                    payout_id_type,
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
    connector: &'static str,
) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
    metrics::RESPONSE_DESERIALIZATION_FAILURE
        .add(1, router_env::metric_attributes!(("connector", connector)));

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
                issuer_error_code: None,
                issuer_error_message: None,
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
            Some("1xx") => metrics::CONNECTOR_HTTP_STATUS_CODE_1XX_COUNT.add(1, &[]),
            Some("2xx") => metrics::CONNECTOR_HTTP_STATUS_CODE_2XX_COUNT.add(1, &[]),
            Some("3xx") => metrics::CONNECTOR_HTTP_STATUS_CODE_3XX_COUNT.add(1, &[]),
            Some("4xx") => metrics::CONNECTOR_HTTP_STATUS_CODE_4XX_COUNT.add(1, &[]),
            Some("5xx") => metrics::CONNECTOR_HTTP_STATUS_CODE_5XX_COUNT.add(1, &[]),
            _ => logger::info!("Skip metrics as invalid http status code received from connector"),
        };
    } else {
        logger::info!("Skip metrics as no http status code received from connector")
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[async_trait::async_trait]
pub trait CustomerAddress {
    async fn get_address_update(
        &self,
        state: &SessionState,
        address_details: payments::AddressDetails,
        key: &[u8],
        storage_scheme: storage::enums::MerchantStorageScheme,
        merchant_id: id_type::MerchantId,
    ) -> CustomResult<storage::AddressUpdate, common_utils::errors::CryptoError>;

    async fn get_domain_address(
        &self,
        state: &SessionState,
        address_details: payments::AddressDetails,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
        key: &[u8],
        storage_scheme: storage::enums::MerchantStorageScheme,
    ) -> CustomResult<domain::CustomerAddress, common_utils::errors::CryptoError>;
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[async_trait::async_trait]
impl CustomerAddress for api_models::customers::CustomerRequest {
    async fn get_address_update(
        &self,
        state: &SessionState,
        address_details: payments::AddressDetails,
        key: &[u8],
        storage_scheme: storage::enums::MerchantStorageScheme,
        merchant_id: id_type::MerchantId,
    ) -> CustomResult<storage::AddressUpdate, common_utils::errors::CryptoError> {
        let encrypted_data = crypto_operation(
            &state.into(),
            type_name!(storage::Address),
            CryptoOperation::BatchEncrypt(domain::FromRequestEncryptableAddress::to_encryptable(
                domain::FromRequestEncryptableAddress {
                    line1: address_details.line1.clone(),
                    line2: address_details.line2.clone(),
                    line3: address_details.line3.clone(),
                    state: address_details.state.clone(),
                    first_name: address_details.first_name.clone(),
                    last_name: address_details.last_name.clone(),
                    zip: address_details.zip.clone(),
                    phone_number: self.phone.clone(),
                    email: self
                        .email
                        .as_ref()
                        .map(|a| a.clone().expose().switch_strategy()),
                },
            )),
            Identifier::Merchant(merchant_id.to_owned()),
            key,
        )
        .await
        .and_then(|val| val.try_into_batchoperation())?;

        let encryptable_address =
            domain::FromRequestEncryptableAddress::from_encryptable(encrypted_data)
                .change_context(common_utils::errors::CryptoError::EncodingFailed)?;

        Ok(storage::AddressUpdate::Update {
            city: address_details.city,
            country: address_details.country,
            line1: encryptable_address.line1,
            line2: encryptable_address.line2,
            line3: encryptable_address.line3,
            zip: encryptable_address.zip,
            state: encryptable_address.state,
            first_name: encryptable_address.first_name,
            last_name: encryptable_address.last_name,
            phone_number: encryptable_address.phone_number,
            country_code: self.phone_country_code.clone(),
            updated_by: storage_scheme.to_string(),
            email: encryptable_address.email.map(|email| {
                let encryptable: Encryptable<masking::Secret<String, pii::EmailStrategy>> =
                    Encryptable::new(
                        email.clone().into_inner().switch_strategy(),
                        email.into_encrypted(),
                    );
                encryptable
            }),
        })
    }

    async fn get_domain_address(
        &self,
        state: &SessionState,
        address_details: payments::AddressDetails,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
        key: &[u8],
        storage_scheme: storage::enums::MerchantStorageScheme,
    ) -> CustomResult<domain::CustomerAddress, common_utils::errors::CryptoError> {
        let encrypted_data = crypto_operation(
            &state.into(),
            type_name!(storage::Address),
            CryptoOperation::BatchEncrypt(domain::FromRequestEncryptableAddress::to_encryptable(
                domain::FromRequestEncryptableAddress {
                    line1: address_details.line1.clone(),
                    line2: address_details.line2.clone(),
                    line3: address_details.line3.clone(),
                    state: address_details.state.clone(),
                    first_name: address_details.first_name.clone(),
                    last_name: address_details.last_name.clone(),
                    zip: address_details.zip.clone(),
                    phone_number: self.phone.clone(),
                    email: self
                        .email
                        .as_ref()
                        .map(|a| a.clone().expose().switch_strategy()),
                },
            )),
            Identifier::Merchant(merchant_id.to_owned()),
            key,
        )
        .await
        .and_then(|val| val.try_into_batchoperation())?;

        let encryptable_address =
            domain::FromRequestEncryptableAddress::from_encryptable(encrypted_data)
                .change_context(common_utils::errors::CryptoError::EncodingFailed)?;

        let address = domain::Address {
            city: address_details.city,
            country: address_details.country,
            line1: encryptable_address.line1,
            line2: encryptable_address.line2,
            line3: encryptable_address.line3,
            zip: encryptable_address.zip,
            state: encryptable_address.state,
            first_name: encryptable_address.first_name,
            last_name: encryptable_address.last_name,
            phone_number: encryptable_address.phone_number,
            country_code: self.phone_country_code.clone(),
            merchant_id: merchant_id.to_owned(),
            address_id: generate_id(consts::ID_LENGTH, "add"),
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            updated_by: storage_scheme.to_string(),
            email: encryptable_address.email.map(|email| {
                let encryptable: Encryptable<masking::Secret<String, pii::EmailStrategy>> =
                    Encryptable::new(
                        email.clone().into_inner().switch_strategy(),
                        email.into_encrypted(),
                    );
                encryptable
            }),
        };

        Ok(domain::CustomerAddress {
            address,
            customer_id: customer_id.to_owned(),
        })
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[async_trait::async_trait]
impl CustomerAddress for api_models::customers::CustomerUpdateRequest {
    async fn get_address_update(
        &self,
        state: &SessionState,
        address_details: payments::AddressDetails,
        key: &[u8],
        storage_scheme: storage::enums::MerchantStorageScheme,
        merchant_id: id_type::MerchantId,
    ) -> CustomResult<storage::AddressUpdate, common_utils::errors::CryptoError> {
        let encrypted_data = crypto_operation(
            &state.into(),
            type_name!(storage::Address),
            CryptoOperation::BatchEncrypt(domain::FromRequestEncryptableAddress::to_encryptable(
                domain::FromRequestEncryptableAddress {
                    line1: address_details.line1.clone(),
                    line2: address_details.line2.clone(),
                    line3: address_details.line3.clone(),
                    state: address_details.state.clone(),
                    first_name: address_details.first_name.clone(),
                    last_name: address_details.last_name.clone(),
                    zip: address_details.zip.clone(),
                    phone_number: self.phone.clone(),
                    email: self
                        .email
                        .as_ref()
                        .map(|a| a.clone().expose().switch_strategy()),
                },
            )),
            Identifier::Merchant(merchant_id.to_owned()),
            key,
        )
        .await
        .and_then(|val| val.try_into_batchoperation())?;

        let encryptable_address =
            domain::FromRequestEncryptableAddress::from_encryptable(encrypted_data)
                .change_context(common_utils::errors::CryptoError::EncodingFailed)?;
        Ok(storage::AddressUpdate::Update {
            city: address_details.city,
            country: address_details.country,
            line1: encryptable_address.line1,
            line2: encryptable_address.line2,
            line3: encryptable_address.line3,
            zip: encryptable_address.zip,
            state: encryptable_address.state,
            first_name: encryptable_address.first_name,
            last_name: encryptable_address.last_name,
            phone_number: encryptable_address.phone_number,
            country_code: self.phone_country_code.clone(),
            updated_by: storage_scheme.to_string(),
            email: encryptable_address.email.map(|email| {
                let encryptable: Encryptable<masking::Secret<String, pii::EmailStrategy>> =
                    Encryptable::new(
                        email.clone().into_inner().switch_strategy(),
                        email.into_encrypted(),
                    );
                encryptable
            }),
        })
    }

    async fn get_domain_address(
        &self,
        state: &SessionState,
        address_details: payments::AddressDetails,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
        key: &[u8],
        storage_scheme: storage::enums::MerchantStorageScheme,
    ) -> CustomResult<domain::CustomerAddress, common_utils::errors::CryptoError> {
        let encrypted_data = crypto_operation(
            &state.into(),
            type_name!(storage::Address),
            CryptoOperation::BatchEncrypt(domain::FromRequestEncryptableAddress::to_encryptable(
                domain::FromRequestEncryptableAddress {
                    line1: address_details.line1.clone(),
                    line2: address_details.line2.clone(),
                    line3: address_details.line3.clone(),
                    state: address_details.state.clone(),
                    first_name: address_details.first_name.clone(),
                    last_name: address_details.last_name.clone(),
                    zip: address_details.zip.clone(),
                    phone_number: self.phone.clone(),
                    email: self
                        .email
                        .as_ref()
                        .map(|a| a.clone().expose().switch_strategy()),
                },
            )),
            Identifier::Merchant(merchant_id.to_owned()),
            key,
        )
        .await
        .and_then(|val| val.try_into_batchoperation())?;

        let encryptable_address =
            domain::FromRequestEncryptableAddress::from_encryptable(encrypted_data)
                .change_context(common_utils::errors::CryptoError::EncodingFailed)?;
        let address = domain::Address {
            city: address_details.city,
            country: address_details.country,
            line1: encryptable_address.line1,
            line2: encryptable_address.line2,
            line3: encryptable_address.line3,
            zip: encryptable_address.zip,
            state: encryptable_address.state,
            first_name: encryptable_address.first_name,
            last_name: encryptable_address.last_name,
            phone_number: encryptable_address.phone_number,
            country_code: self.phone_country_code.clone(),
            merchant_id: merchant_id.to_owned(),
            address_id: generate_id(consts::ID_LENGTH, "add"),
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            updated_by: storage_scheme.to_string(),
            email: encryptable_address.email.map(|email| {
                let encryptable: Encryptable<masking::Secret<String, pii::EmailStrategy>> =
                    Encryptable::new(
                        email.clone().into_inner().switch_strategy(),
                        email.into_encrypted(),
                    );
                encryptable
            }),
        };

        Ok(domain::CustomerAddress {
            address,
            customer_id: customer_id.to_owned(),
        })
    }
}

pub fn add_apple_pay_flow_metrics(
    apple_pay_flow: &Option<domain::ApplePayFlow>,
    connector: Option<String>,
    merchant_id: id_type::MerchantId,
) {
    if let Some(flow) = apple_pay_flow {
        match flow {
            domain::ApplePayFlow::Simplified(_) => metrics::APPLE_PAY_SIMPLIFIED_FLOW.add(
                1,
                router_env::metric_attributes!(
                    (
                        "connector",
                        connector.to_owned().unwrap_or("null".to_string()),
                    ),
                    ("merchant_id", merchant_id.clone()),
                ),
            ),
            domain::ApplePayFlow::Manual => metrics::APPLE_PAY_MANUAL_FLOW.add(
                1,
                router_env::metric_attributes!(
                    (
                        "connector",
                        connector.to_owned().unwrap_or("null".to_string()),
                    ),
                    ("merchant_id", merchant_id.clone()),
                ),
            ),
        }
    }
}

pub fn add_apple_pay_payment_status_metrics(
    payment_attempt_status: enums::AttemptStatus,
    apple_pay_flow: Option<domain::ApplePayFlow>,
    connector: Option<String>,
    merchant_id: id_type::MerchantId,
) {
    if payment_attempt_status == enums::AttemptStatus::Charged {
        if let Some(flow) = apple_pay_flow {
            match flow {
                domain::ApplePayFlow::Simplified(_) => {
                    metrics::APPLE_PAY_SIMPLIFIED_FLOW_SUCCESSFUL_PAYMENT.add(
                        1,
                        router_env::metric_attributes!(
                            (
                                "connector",
                                connector.to_owned().unwrap_or("null".to_string()),
                            ),
                            ("merchant_id", merchant_id.clone()),
                        ),
                    )
                }
                domain::ApplePayFlow::Manual => metrics::APPLE_PAY_MANUAL_FLOW_SUCCESSFUL_PAYMENT
                    .add(
                        1,
                        router_env::metric_attributes!(
                            (
                                "connector",
                                connector.to_owned().unwrap_or("null".to_string()),
                            ),
                            ("merchant_id", merchant_id.clone()),
                        ),
                    ),
            }
        }
    } else if payment_attempt_status == enums::AttemptStatus::Failure {
        if let Some(flow) = apple_pay_flow {
            match flow {
                domain::ApplePayFlow::Simplified(_) => {
                    metrics::APPLE_PAY_SIMPLIFIED_FLOW_FAILED_PAYMENT.add(
                        1,
                        router_env::metric_attributes!(
                            (
                                "connector",
                                connector.to_owned().unwrap_or("null".to_string()),
                            ),
                            ("merchant_id", merchant_id.clone()),
                        ),
                    )
                }
                domain::ApplePayFlow::Manual => metrics::APPLE_PAY_MANUAL_FLOW_FAILED_PAYMENT.add(
                    1,
                    router_env::metric_attributes!(
                        (
                            "connector",
                            connector.to_owned().unwrap_or("null".to_string()),
                        ),
                        ("merchant_id", merchant_id.clone()),
                    ),
                ),
            }
        }
    }
}

pub fn check_if_pull_mechanism_for_external_3ds_enabled_from_connector_metadata(
    metadata: Option<Value>,
) -> bool {
    let external_three_ds_connector_metadata: Option<ExternalThreeDSConnectorMetadata> = metadata
            .parse_value("ExternalThreeDSConnectorMetadata")
            .map_err(|err| logger::warn!(parsing_error=?err,"Error while parsing ExternalThreeDSConnectorMetadata"))
            .ok();
    external_three_ds_connector_metadata
        .and_then(|metadata| metadata.pull_mechanism_for_external_3ds_enabled)
        .unwrap_or(true)
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn trigger_payments_webhook<F, Op, D>(
    merchant_account: domain::MerchantAccount,
    business_profile: domain::Profile,
    key_store: &domain::MerchantKeyStore,
    payment_data: D,
    customer: Option<domain::Customer>,
    state: &SessionState,
    operation: Op,
) -> RouterResult<()>
where
    F: Send + Clone + Sync,
    Op: Debug,
    D: payments_core::OperationSessionGetters<F>,
{
    todo!()
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn trigger_payments_webhook<F, Op, D>(
    merchant_account: domain::MerchantAccount,
    business_profile: domain::Profile,
    key_store: &domain::MerchantKeyStore,
    payment_data: D,
    customer: Option<domain::Customer>,
    state: &SessionState,
    operation: Op,
) -> RouterResult<()>
where
    F: Send + Clone + Sync,
    Op: Debug,
    D: payments_core::OperationSessionGetters<F>,
{
    let status = payment_data.get_payment_intent().status;
    let payment_id = payment_data.get_payment_intent().get_id().to_owned();

    let captures = payment_data
        .get_multiple_capture_data()
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
            | enums::IntentStatus::RequiresMerchantAction
    ) {
        let payments_response = crate::core::payments::transformers::payments_to_payments_response(
            payment_data,
            captures,
            customer,
            services::AuthFlow::Merchant,
            &state.base_url,
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
                            payment_id.get_string_repr().to_owned(),
                            diesel_models::enums::EventObjectType::PaymentDetails,
                            webhooks::OutgoingWebhookContent::PaymentDetails(Box::new(
                                payments_response_json,
                            )),
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

#[cfg(feature = "v1")]
pub async fn trigger_refund_outgoing_webhook(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    refund: &diesel_models::Refund,
    profile_id: id_type::ProfileId,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<()> {
    let refund_status = refund.refund_status;
    if matches!(
        refund_status,
        enums::RefundStatus::Success
            | enums::RefundStatus::Failure
            | enums::RefundStatus::TransactionFailure
    ) {
        let event_type = ForeignFrom::foreign_from(refund_status);
        let refund_response: api_models::refunds::RefundResponse = refund.clone().foreign_into();
        let key_manager_state = &(state).into();
        let refund_id = refund_response.refund_id.clone();
        let business_profile = state
            .store
            .find_business_profile_by_profile_id(key_manager_state, key_store, &profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;
        let cloned_state = state.clone();
        let cloned_key_store = key_store.clone();
        let cloned_merchant_account = merchant_account.clone();
        let primary_object_created_at = refund_response.created_at;
        if let Some(outgoing_event_type) = event_type {
            tokio::spawn(
                async move {
                    Box::pin(webhooks_core::create_event_and_trigger_outgoing_webhook(
                        cloned_state,
                        cloned_merchant_account,
                        business_profile,
                        &cloned_key_store,
                        outgoing_event_type,
                        diesel_models::enums::EventClass::Refunds,
                        refund_id.to_string(),
                        diesel_models::enums::EventObjectType::RefundDetails,
                        webhooks::OutgoingWebhookContent::RefundDetails(Box::new(refund_response)),
                        primary_object_created_at,
                    ))
                    .await
                }
                .in_current_span(),
            );
        } else {
            logger::warn!("Outgoing webhook not sent because of missing event type status mapping");
        };
    }
    Ok(())
}

#[cfg(feature = "v2")]
pub async fn trigger_refund_outgoing_webhook(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    refund: &diesel_models::Refund,
    profile_id: id_type::ProfileId,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<()> {
    todo!()
}

pub fn get_locale_from_header(headers: &actix_web::http::header::HeaderMap) -> String {
    get_header_value_by_key(ACCEPT_LANGUAGE.into(), headers)
        .ok()
        .flatten()
        .map(|val| val.to_string())
        .unwrap_or(common_utils::consts::DEFAULT_LOCALE.to_string())
}
