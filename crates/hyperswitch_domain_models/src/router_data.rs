use std::{collections::HashMap, marker::PhantomData};

use common_utils::{id_type, types::MinorUnit};
use masking::Secret;

use crate::{payment_address::PaymentAddress, payment_method_data};

#[derive(Debug, Clone)]
pub struct RouterData<Flow, Request, Response> {
    pub flow: PhantomData<Flow>,
    pub merchant_id: String,
    pub customer_id: Option<id_type::CustomerId>,
    pub connector_customer: Option<String>,
    pub connector: String,
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

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct AccessToken {
    pub token: Secret<String>,
    pub expires: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum PaymentMethodToken {
    Token(Secret<String>),
    ApplePayDecrypt(Box<ApplePayPredecryptData>),
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

#[derive(Debug, Default, Clone)]
pub struct RecurringMandatePaymentData {
    pub payment_method_type: Option<common_enums::enums::PaymentMethodType>, //required for making recurring payment using saved payment method through stripe
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<common_enums::enums::Currency>,
}

#[derive(Debug, Clone)]
pub struct PaymentMethodBalance {
    pub amount: i64,
    pub currency: common_enums::enums::Currency,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorResponseData {
    pub additional_payment_method_data: Option<AdditionalPaymentMethodConnectorResponse>,
}

impl ConnectorResponseData {
    pub fn with_additional_payment_method_data(
        additional_payment_method_data: AdditionalPaymentMethodConnectorResponse,
    ) -> Self {
        Self {
            additional_payment_method_data: Some(additional_payment_method_data),
        }
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
