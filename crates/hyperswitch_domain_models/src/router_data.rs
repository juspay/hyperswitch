use std::{collections::HashMap, marker::PhantomData};

use common_utils::{
    errors::IntegrityCheckError,
    ext_traits::{OptionExt, ValueExt},
    id_type,
    types::{self, MinorUnit},
};
use error_stack::ResultExt;
use masking::{ExposeInterface, Secret};

use crate::{payment_address::PaymentAddress, payment_method_data, payments};

#[derive(Debug, Clone)]
pub struct RouterData<Flow, Request, Response> {
    pub flow: PhantomData<Flow>,
    pub merchant_id: id_type::MerchantId,
    pub customer_id: Option<id_type::CustomerId>,
    pub connector_customer: Option<String>,
    pub connector: String,
    // TODO: This should be a PaymentId type.
    // Make this change after all the connector dependency has been removed from connectors
    pub payment_id: String,
    pub attempt_id: String,
    pub status: common_enums::enums::AttemptStatus,
    pub payment_method: common_enums::enums::PaymentMethod,
    pub connector_auth_type: ConnectorAuthType,
    pub description: Option<String>,
    pub return_url: Option<String>,
    pub address: PaymentAddress,
    pub auth_type: common_enums::enums::AuthenticationType,
    pub connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    pub connector_wallets_details: Option<common_utils::pii::SecretSerdeValue>,
    pub amount_captured: Option<i64>,
    pub access_token: Option<AccessToken>,
    pub session_token: Option<String>,
    pub reference_id: Option<String>,
    pub payment_method_token: Option<PaymentMethodToken>,
    pub recurring_mandate_payment_data: Option<RecurringMandatePaymentData>,
    pub preprocessing_id: Option<String>,
    /// This is the balance amount for gift cards or voucher
    pub payment_method_balance: Option<PaymentMethodBalance>,

    ///for switching between two different versions of the same connector
    pub connector_api_version: Option<String>,

    /// Contains flow-specific data required to construct a request and send it to the connector.
    pub request: Request,

    /// Contains flow-specific data that the connector responds with.
    pub response: Result<Response, ErrorResponse>,

    /// Contains a reference ID that should be sent in the connector request
    pub connector_request_reference_id: String,

    #[cfg(feature = "payouts")]
    /// Contains payout method data
    pub payout_method_data: Option<api_models::payouts::PayoutMethodData>,

    #[cfg(feature = "payouts")]
    /// Contains payout's quote ID
    pub quote_id: Option<String>,

    pub test_mode: Option<bool>,
    pub connector_http_status_code: Option<u16>,
    pub external_latency: Option<u128>,
    /// Contains apple pay flow type simplified or manual
    pub apple_pay_flow: Option<payment_method_data::ApplePayFlow>,

    pub frm_metadata: Option<common_utils::pii::SecretSerdeValue>,

    pub dispute_id: Option<String>,
    pub refund_id: Option<String>,

    /// This field is used to store various data regarding the response from connector
    pub connector_response: Option<ConnectorResponseData>,
    pub payment_method_status: Option<common_enums::PaymentMethodStatus>,

    // minor amount for amount framework
    pub minor_amount_captured: Option<MinorUnit>,

    pub integrity_check: Result<(), IntegrityCheckError>,

    pub additional_merchant_data: Option<api_models::admin::AdditionalMerchantData>,

    pub header_payload: Option<payments::HeaderPayload>,

    pub connector_mandate_request_reference_id: Option<String>,

    pub authentication_id: Option<String>,
    /// Contains the type of sca exemption required for the transaction
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
}

// Different patterns of authentication.
#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "auth_type")]
pub enum ConnectorAuthType {
    TemporaryAuth,
    HeaderKey {
        api_key: Secret<String>,
    },
    BodyKey {
        api_key: Secret<String>,
        key1: Secret<String>,
    },
    SignatureKey {
        api_key: Secret<String>,
        key1: Secret<String>,
        api_secret: Secret<String>,
    },
    MultiAuthKey {
        api_key: Secret<String>,
        key1: Secret<String>,
        api_secret: Secret<String>,
        key2: Secret<String>,
    },
    CurrencyAuthKey {
        auth_key_map: HashMap<common_enums::enums::Currency, common_utils::pii::SecretSerdeValue>,
    },
    CertificateAuth {
        certificate: Secret<String>,
        private_key: Secret<String>,
    },
    #[default]
    NoKey,
}

impl ConnectorAuthType {
    pub fn from_option_secret_value(
        value: Option<common_utils::pii::SecretSerdeValue>,
    ) -> common_utils::errors::CustomResult<Self, common_utils::errors::ParsingError> {
        value
            .parse_value::<Self>("ConnectorAuthType")
            .change_context(common_utils::errors::ParsingError::StructParseFailure(
                "ConnectorAuthType",
            ))
    }

    pub fn from_secret_value(
        value: common_utils::pii::SecretSerdeValue,
    ) -> common_utils::errors::CustomResult<Self, common_utils::errors::ParsingError> {
        value
            .parse_value::<Self>("ConnectorAuthType")
            .change_context(common_utils::errors::ParsingError::StructParseFailure(
                "ConnectorAuthType",
            ))
    }

    // show only first and last two digits of the key and mask others with *
    // mask the entire key if it's length is less than or equal to 4
    fn mask_key(&self, key: String) -> Secret<String> {
        let key_len = key.len();
        let masked_key = if key_len <= 4 {
            "*".repeat(key_len)
        } else {
            // Show the first two and last two characters, mask the rest with '*'
            let mut masked_key = String::new();
            let key_len = key.len();
            // Iterate through characters by their index
            for (index, character) in key.chars().enumerate() {
                if index < 2 || index >= key_len - 2 {
                    masked_key.push(character); // Keep the first two and last two characters
                } else {
                    masked_key.push('*'); // Mask the middle characters
                }
            }
            masked_key
        };
        Secret::new(masked_key)
    }

    // Mask the keys in the auth_type
    pub fn get_masked_keys(&self) -> Self {
        match self {
            Self::TemporaryAuth => Self::TemporaryAuth,
            Self::NoKey => Self::NoKey,
            Self::HeaderKey { api_key } => Self::HeaderKey {
                api_key: self.mask_key(api_key.clone().expose()),
            },
            Self::BodyKey { api_key, key1 } => Self::BodyKey {
                api_key: self.mask_key(api_key.clone().expose()),
                key1: self.mask_key(key1.clone().expose()),
            },
            Self::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Self::SignatureKey {
                api_key: self.mask_key(api_key.clone().expose()),
                key1: self.mask_key(key1.clone().expose()),
                api_secret: self.mask_key(api_secret.clone().expose()),
            },
            Self::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Self::MultiAuthKey {
                api_key: self.mask_key(api_key.clone().expose()),
                key1: self.mask_key(key1.clone().expose()),
                api_secret: self.mask_key(api_secret.clone().expose()),
                key2: self.mask_key(key2.clone().expose()),
            },
            Self::CurrencyAuthKey { auth_key_map } => Self::CurrencyAuthKey {
                auth_key_map: auth_key_map.clone(),
            },
            Self::CertificateAuth {
                certificate,
                private_key,
            } => Self::CertificateAuth {
                certificate: self.mask_key(certificate.clone().expose()),
                private_key: self.mask_key(private_key.clone().expose()),
            },
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct AccessToken {
    pub token: Secret<String>,
    pub expires: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum PaymentMethodToken {
    Token(Secret<String>),
    ApplePayDecrypt(Box<ApplePayPredecryptData>),
    PazeDecrypt(Box<PazeDecryptedData>),
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayPredecryptData {
    pub application_primary_account_number: Secret<String>,
    pub application_expiration_date: String,
    pub currency_code: String,
    pub transaction_amount: i64,
    pub device_manufacturer_identifier: Secret<String>,
    pub payment_data_type: Secret<String>,
    pub payment_data: ApplePayCryptogramData,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayCryptogramData {
    pub online_payment_cryptogram: Secret<String>,
    pub eci_indicator: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PazeDecryptedData {
    pub client_id: Secret<String>,
    pub profile_id: String,
    pub token: PazeToken,
    pub payment_card_network: common_enums::enums::CardNetwork,
    pub dynamic_data: Vec<PazeDynamicData>,
    pub billing_address: PazeAddress,
    pub consumer: PazeConsumer,
    pub eci: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PazeToken {
    pub payment_token: cards::CardNumber,
    pub token_expiration_month: Secret<String>,
    pub token_expiration_year: Secret<String>,
    pub payment_account_reference: Secret<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PazeDynamicData {
    pub dynamic_data_value: Option<Secret<String>>,
    pub dynamic_data_type: Option<String>,
    pub dynamic_data_expiration: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PazeAddress {
    pub name: Option<Secret<String>>,
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub line3: Option<Secret<String>>,
    pub city: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub country_code: Option<common_enums::enums::CountryAlpha2>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PazeConsumer {
    // This is consumer data not customer data.
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub full_name: Secret<String>,
    pub email_address: common_utils::pii::Email,
    pub mobile_number: Option<PazePhoneNumber>,
    pub country_code: Option<common_enums::enums::CountryAlpha2>,
    pub language_code: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PazePhoneNumber {
    pub country_code: Secret<String>,
    pub phone_number: Secret<String>,
}

#[derive(Debug, Default, Clone)]
pub struct RecurringMandatePaymentData {
    pub payment_method_type: Option<common_enums::enums::PaymentMethodType>, //required for making recurring payment using saved payment method through stripe
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<common_enums::enums::Currency>,
    pub mandate_metadata: Option<common_utils::pii::SecretSerdeValue>,
}

#[derive(Debug, Clone)]
pub struct PaymentMethodBalance {
    pub amount: MinorUnit,
    pub currency: common_enums::enums::Currency,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorResponseData {
    pub additional_payment_method_data: Option<AdditionalPaymentMethodConnectorResponse>,
    extended_authorization_response_data: Option<ExtendedAuthorizationResponseData>,
}

impl ConnectorResponseData {
    pub fn with_additional_payment_method_data(
        additional_payment_method_data: AdditionalPaymentMethodConnectorResponse,
    ) -> Self {
        Self {
            additional_payment_method_data: Some(additional_payment_method_data),
            extended_authorization_response_data: None,
        }
    }
    pub fn get_extended_authorization_response_data(
        &self,
    ) -> Option<&ExtendedAuthorizationResponseData> {
        self.extended_authorization_response_data.as_ref()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AdditionalPaymentMethodConnectorResponse {
    Card {
        /// Details regarding the authentication details of the connector, if this is a 3ds payment.
        authentication_data: Option<serde_json::Value>,
        /// Various payment checks that are done for a payment
        payment_checks: Option<serde_json::Value>,
    },
    PayLater {
        klarna_sdk: Option<KlarnaSdkResponse>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtendedAuthorizationResponseData {
    pub extended_authentication_applied: Option<types::ExtendedAuthorizationAppliedBool>,
    pub capture_before: Option<time::PrimitiveDateTime>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KlarnaSdkResponse {
    pub payment_type: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub status_code: u16,
    pub attempt_status: Option<common_enums::enums::AttemptStatus>,
    pub connector_transaction_id: Option<String>,
}

impl Default for ErrorResponse {
    fn default() -> Self {
        Self {
            code: "HE_00".to_string(),
            message: "Something went wrong".to_string(),
            reason: None,
            status_code: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            attempt_status: None,
            connector_transaction_id: None,
        }
    }
}

impl ErrorResponse {
    pub fn get_not_implemented() -> Self {
        Self {
            code: "IR_00".to_string(),
            message: "This API is under development and will be made available soon.".to_string(),
            reason: None,
            status_code: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            attempt_status: None,
            connector_transaction_id: None,
        }
    }
}

#[cfg(feature = "v2")]
use crate::{
    payments::{
        payment_attempt::{ErrorDetails, PaymentAttemptUpdate},
        payment_intent::PaymentIntentUpdate,
    },
    router_flow_types, router_request_types, router_response_types,
};

/// Get updatable trakcer objects of payment intent and payment attempt
#[cfg(feature = "v2")]
pub trait TrackerPostUpdateObjects<Flow, FlowRequest> {
    fn get_payment_intent_update(
        &self,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentIntentUpdate;
    fn get_payment_attempt_update(
        &self,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentAttemptUpdate;
}

#[cfg(feature = "v2")]
impl
    TrackerPostUpdateObjects<
        router_flow_types::Authorize,
        router_request_types::PaymentsAuthorizeData,
    >
    for RouterData<
        router_flow_types::Authorize,
        router_request_types::PaymentsAuthorizeData,
        router_response_types::PaymentsResponseData,
    >
{
    fn get_payment_intent_update(
        &self,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentIntentUpdate {
        match self.response {
            Ok(ref _response) => PaymentIntentUpdate::ConfirmIntentPostUpdate {
                status: common_enums::IntentStatus::from(self.status),
                updated_by: storage_scheme.to_string(),
            },
            Err(ref error) => PaymentIntentUpdate::ConfirmIntentPostUpdate {
                status: error
                    .attempt_status
                    .map(common_enums::IntentStatus::from)
                    .unwrap_or(common_enums::IntentStatus::Failed),
                updated_by: storage_scheme.to_string(),
            },
        }
    }

    fn get_payment_attempt_update(
        &self,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentAttemptUpdate {
        match self.response {
            Ok(ref response_router_data) => match response_router_data {
                router_response_types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data,
                    mandate_reference,
                    connector_metadata,
                    network_txn_id,
                    connector_response_reference_id,
                    incremental_authorization_allowed,
                    charge_id,
                } => {
                    let attempt_status = self.status;
                    let connector_payment_id = match resource_id {
                        router_request_types::ResponseId::NoResponseId => None,
                        router_request_types::ResponseId::ConnectorTransactionId(id)
                        | router_request_types::ResponseId::EncodedData(id) => Some(id.to_owned()),
                    };

                    PaymentAttemptUpdate::ConfirmIntentResponse {
                        status: attempt_status,
                        connector_payment_id,
                        updated_by: storage_scheme.to_string(),
                        redirection_data: *redirection_data.clone(),
                        connector_metadata: connector_metadata.clone().map(Secret::new),
                    }
                }
                router_response_types::PaymentsResponseData::MultipleCaptureResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::SessionResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::SessionTokenResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::TransactionUnresolvedResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::TokenizationResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::ConnectorCustomerResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PreProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::IncrementalAuthorizationResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PostProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::SessionUpdateResponse { .. } => {
                    todo!()
                }
            },
            Err(ref error_response) => {
                let ErrorResponse {
                    code,
                    message,
                    reason,
                    status_code: _,
                    attempt_status,
                    connector_transaction_id,
                } = error_response.clone();
                let attempt_status = attempt_status.unwrap_or(self.status);

                let error_details = ErrorDetails {
                    code,
                    message,
                    reason,
                    unified_code: None,
                    unified_message: None,
                };

                PaymentAttemptUpdate::ErrorUpdate {
                    status: attempt_status,
                    error: error_details,
                    connector_payment_id: connector_transaction_id,
                    updated_by: storage_scheme.to_string(),
                }
            }
        }
    }
}

#[cfg(feature = "v2")]
impl TrackerPostUpdateObjects<router_flow_types::PSync, router_request_types::PaymentsSyncData>
    for RouterData<
        router_flow_types::PSync,
        router_request_types::PaymentsSyncData,
        router_response_types::PaymentsResponseData,
    >
{
    fn get_payment_intent_update(
        &self,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentIntentUpdate {
        match self.response {
            Ok(ref _response) => PaymentIntentUpdate::SyncUpdate {
                status: common_enums::IntentStatus::from(self.status),
                updated_by: storage_scheme.to_string(),
            },
            Err(ref error) => PaymentIntentUpdate::SyncUpdate {
                status: error
                    .attempt_status
                    .map(common_enums::IntentStatus::from)
                    .unwrap_or(common_enums::IntentStatus::Failed),
                updated_by: storage_scheme.to_string(),
            },
        }
    }

    fn get_payment_attempt_update(
        &self,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> PaymentAttemptUpdate {
        match self.response {
            Ok(ref response_router_data) => match response_router_data {
                router_response_types::PaymentsResponseData::TransactionResponse {
                    resource_id,
                    redirection_data,
                    mandate_reference,
                    connector_metadata,
                    network_txn_id,
                    connector_response_reference_id,
                    incremental_authorization_allowed,
                    charge_id,
                } => {
                    let attempt_status = self.status;
                    let connector_payment_id = match resource_id {
                        router_request_types::ResponseId::NoResponseId => None,
                        router_request_types::ResponseId::ConnectorTransactionId(id)
                        | router_request_types::ResponseId::EncodedData(id) => Some(id.to_owned()),
                    };

                    PaymentAttemptUpdate::SyncUpdate {
                        status: attempt_status,
                        updated_by: storage_scheme.to_string(),
                    }
                }
                router_response_types::PaymentsResponseData::MultipleCaptureResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::SessionResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::SessionTokenResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::TransactionUnresolvedResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::TokenizationResponse { .. } => todo!(),
                router_response_types::PaymentsResponseData::ConnectorCustomerResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PreProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::IncrementalAuthorizationResponse {
                    ..
                } => todo!(),
                router_response_types::PaymentsResponseData::PostProcessingResponse { .. } => {
                    todo!()
                }
                router_response_types::PaymentsResponseData::SessionUpdateResponse { .. } => {
                    todo!()
                }
            },
            Err(ref error_response) => {
                let ErrorResponse {
                    code,
                    message,
                    reason,
                    status_code: _,
                    attempt_status,
                    connector_transaction_id,
                } = error_response.clone();
                let attempt_status = attempt_status.unwrap_or(self.status);

                let error_details = ErrorDetails {
                    code,
                    message,
                    reason,
                    unified_code: None,
                    unified_message: None,
                };

                PaymentAttemptUpdate::ErrorUpdate {
                    status: attempt_status,
                    error: error_details,
                    connector_payment_id: connector_transaction_id,
                    updated_by: storage_scheme.to_string(),
                }
            }
        }
    }
}
