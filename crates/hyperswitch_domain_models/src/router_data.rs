use std::{collections::HashMap, marker::PhantomData};

use masking::Secret;

use crate::payment_address::PaymentAddress;

#[derive(Debug, Clone)]
pub struct RouterData<Flow, Request, Response> {
    pub flow: PhantomData<Flow>,
    pub merchant_id: String,
    pub customer_id: Option<String>,
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
    pub apple_pay_flow: Option<common_enums::enums::ApplePayFlow>,

    pub frm_metadata: Option<serde_json::Value>,

    pub dispute_id: Option<String>,
    pub refund_id: Option<String>,

    /// This field is used to store various data regarding the response from connector
    pub connector_response: Option<ConnectorResponseData>,
    pub payment_method_status: Option<common_enums::PaymentMethodStatus>,
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
    Token(String),
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

// #[derive(Debug, Clone)]
// pub struct PaymentsAuthorizeData {
//     pub payment_method_data: domain::payments::PaymentMethodData,
//     /// total amount (original_amount + surcharge_amount + tax_on_surcharge_amount)
//     /// If connector supports separate field for surcharge amount, consider using below functions defined on `PaymentsAuthorizeData` to fetch original amount and surcharge amount separately
//     /// ```
//     /// get_original_amount()
//     /// get_surcharge_amount()
//     /// get_tax_on_surcharge_amount()
//     /// get_total_surcharge_amount() // returns surcharge_amount + tax_on_surcharge_amount
//     /// ```
//     pub amount: i64,
//     pub email: Option<Email>,
//     pub customer_name: Option<Secret<String>>,
//     pub currency: common_enums::enums::Currency,
//     pub confirm: bool,
//     pub statement_descriptor_suffix: Option<String>,
//     pub statement_descriptor: Option<String>,
//     pub capture_method: Option<common_enums::enums::CaptureMethod>,
//     pub router_return_url: Option<String>,
//     pub webhook_url: Option<String>,
//     pub complete_authorize_url: Option<String>,
//     // Mandates
//     pub setup_future_usage: Option<common_enums::enums::FutureUsage>,
//     pub mandate_id: Option<api_models::payments::MandateIds>,
//     pub off_session: Option<bool>,
//     pub customer_acceptance: Option<CustomerAcceptance>,
//     pub setup_mandate_details: Option<MandateData>,
//     pub browser_info: Option<BrowserInformation>,
//     pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
//     pub order_category: Option<String>,
//     pub session_token: Option<String>,
//     pub enrolled_for_3ds: bool,
//     pub related_transaction_id: Option<String>,
//     pub payment_experience: Option<common_enums::enums::PaymentExperience>,
//     pub payment_method_type: Option<common_enums::enums::PaymentMethodType>,
//     pub surcharge_details: Option<types::SurchargeDetails>,
//     pub customer_id: Option<String>,
//     pub request_incremental_authorization: bool,
//     pub metadata: Option<pii::SecretSerdeValue>,
//     pub authentication_data: Option<types::AuthenticationData>,
// }

// #[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
// pub struct BrowserInformation {
//     pub color_depth: Option<u8>,
//     pub java_enabled: Option<bool>,
//     pub java_script_enabled: Option<bool>,
//     pub language: Option<String>,
//     pub screen_height: Option<u32>,
//     pub screen_width: Option<u32>,
//     pub time_zone: Option<i32>,
//     pub ip_address: Option<std::net::IpAddr>,
//     pub accept_header: Option<String>,
//     pub user_agent: Option<String>,
// }

// #[derive(Debug, Clone, Default, Serialize)]
// pub enum ResponseId {
//     ConnectorTransactionId(String),
//     EncodedData(String),
//     #[default]
//     NoResponseId,
// }

// #[derive(Debug, Clone)]
// pub enum PaymentsResponseData {
//     TransactionResponse {
//         resource_id: ResponseId,
//         redirection_data: Option<services::RedirectForm>,
//         mandate_reference: Option<MandateReference>,
//         connector_metadata: Option<serde_json::Value>,
//         network_txn_id: Option<String>,
//         connector_response_reference_id: Option<String>,
//         incremental_authorization_allowed: Option<bool>,
//     },
//     MultipleCaptureResponse {
//         // pending_capture_id_list: Vec<String>,
//         capture_sync_response_list: HashMap<String, CaptureSyncResponse>,
//     },
//     SessionResponse {
//         session_token: SessionToken,
//     },
//     SessionTokenResponse {
//         session_token: String,
//     },
//     TransactionUnresolvedResponse {
//         resource_id: ResponseId,
//         //to add more info on cypto response, like `unresolved` reason(overpaid, underpaid, delayed)
//         reason: Option<UnresolvedResponseReason>,
//         connector_response_reference_id: Option<String>,
//     },
//     TokenizationResponse {
//         token: String,
//     },

//     ConnectorCustomerResponse {
//         connector_customer_id: String,
//     },

//     ThreeDSEnrollmentResponse {
//         enrolled_v2: bool,
//         related_transaction_id: Option<String>,
//     },
//     PreProcessingResponse {
//         pre_processing_id: PreprocessingResponseId,
//         connector_metadata: Option<serde_json::Value>,
//         session_token: Option<SessionToken>,
//         connector_response_reference_id: Option<String>,
//     },
//     IncrementalAuthorizationResponse {
//         status: common_enums::AuthorizationStatus,
//         connector_authorization_id: Option<String>,
//         error_code: Option<String>,
//         error_message: Option<String>,
//     },
// }

// #[derive(serde::Serialize, Debug, Clone)]
// pub struct MandateReference {
//     pub connector_mandate_id: Option<String>,
//     pub payment_method_id: Option<String>,
// }

// #[derive(Debug, Clone)]
// pub enum CaptureSyncResponse {
//     Success {
//         resource_id: ResponseId,
//         status: common_enums::enums::AttemptStatus,
//         connector_response_reference_id: Option<String>,
//         amount: Option<i64>,
//     },
//     Error {
//         code: String,
//         message: String,
//         reason: Option<String>,
//         status_code: u16,
//         amount: Option<i64>,
//     },
// }

// #[derive(Debug, Clone)]
// pub enum PreprocessingResponseId {
//     PreProcessingId(String),
//     ConnectorTransactionId(String),
// }
