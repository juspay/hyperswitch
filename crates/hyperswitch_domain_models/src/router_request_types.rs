pub mod authentication;
pub mod fraud_check;
use api_models::payments::RequestSurchargeDetails;
use common_utils::{consts, errors, pii};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use masking::Secret;
use serde::Serialize;
use serde_with::serde_as;

use super::payment_method_data::PaymentMethodData;
use crate::{errors::api_error_response, mandates, payments, router_data};
#[derive(Debug, Clone)]
pub struct PaymentsAuthorizeData {
    pub payment_method_data: PaymentMethodData,
    /// total amount (original_amount + surcharge_amount + tax_on_surcharge_amount)
    /// If connector supports separate field for surcharge amount, consider using below functions defined on `PaymentsAuthorizeData` to fetch original amount and surcharge amount separately
    /// ```
    /// get_original_amount()
    /// get_surcharge_amount()
    /// get_tax_on_surcharge_amount()
    /// get_total_surcharge_amount() // returns surcharge_amount + tax_on_surcharge_amount
    /// ```
    pub amount: i64,
    pub email: Option<pii::Email>,
    pub customer_name: Option<Secret<String>>,
    pub currency: storage_enums::Currency,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub statement_descriptor: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub router_return_url: Option<String>,
    pub webhook_url: Option<String>,
    pub complete_authorize_url: Option<String>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub off_session: Option<bool>,
    pub customer_acceptance: Option<mandates::CustomerAcceptance>,
    pub setup_mandate_details: Option<mandates::MandateData>,
    pub browser_info: Option<BrowserInformation>,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
    pub order_category: Option<String>,
    pub session_token: Option<String>,
    pub enrolled_for_3ds: bool,
    pub related_transaction_id: Option<String>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub surcharge_details: Option<SurchargeDetails>,
    pub customer_id: Option<String>,
    pub request_incremental_authorization: bool,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub authentication_data: Option<AuthenticationData>,
}

#[derive(Debug, Clone, Default)]
pub struct PaymentsCaptureData {
    pub amount_to_capture: i64,
    pub currency: storage_enums::Currency,
    pub connector_transaction_id: String,
    pub payment_amount: i64,
    pub multiple_capture_data: Option<MultipleCaptureRequestData>,
    pub connector_meta: Option<serde_json::Value>,
    pub browser_info: Option<BrowserInformation>,
    pub metadata: Option<pii::SecretSerdeValue>,
    // This metadata is used to store the metadata shared during the payment intent request.
}

#[derive(Debug, Clone, Default)]
pub struct PaymentsIncrementalAuthorizationData {
    pub total_amount: i64,
    pub additional_amount: i64,
    pub currency: storage_enums::Currency,
    pub reason: Option<String>,
    pub connector_transaction_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct MultipleCaptureRequestData {
    pub capture_sequence: i16,
    pub capture_reference: String,
}

#[derive(Debug, Clone)]
pub struct AuthorizeSessionTokenData {
    pub amount_to_capture: Option<i64>,
    pub currency: storage_enums::Currency,
    pub connector_transaction_id: String,
    pub amount: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ConnectorCustomerData {
    pub description: Option<String>,
    pub email: Option<pii::Email>,
    pub phone: Option<Secret<String>>,
    pub name: Option<Secret<String>>,
    pub preprocessing_id: Option<String>,
    pub payment_method_data: PaymentMethodData,
}

#[derive(Debug, Clone)]
pub struct PaymentMethodTokenizationData {
    pub payment_method_data: PaymentMethodData,
    pub browser_info: Option<BrowserInformation>,
    pub currency: storage_enums::Currency,
    pub amount: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PaymentsPreProcessingData {
    pub payment_method_data: Option<PaymentMethodData>,
    pub amount: Option<i64>,
    pub email: Option<pii::Email>,
    pub currency: Option<storage_enums::Currency>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub setup_mandate_details: Option<mandates::MandateData>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
    pub router_return_url: Option<String>,
    pub webhook_url: Option<String>,
    pub complete_authorize_url: Option<String>,
    pub surcharge_details: Option<SurchargeDetails>,
    pub browser_info: Option<BrowserInformation>,
    pub connector_transaction_id: Option<String>,
    pub redirect_response: Option<CompleteAuthorizeRedirectResponse>,
}

#[derive(Debug, Clone)]
pub struct CompleteAuthorizeData {
    pub payment_method_data: Option<PaymentMethodData>,
    pub amount: i64,
    pub email: Option<pii::Email>,
    pub currency: storage_enums::Currency,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    // Mandates
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<mandates::MandateData>,
    pub redirect_response: Option<CompleteAuthorizeRedirectResponse>,
    pub browser_info: Option<BrowserInformation>,
    pub connector_transaction_id: Option<String>,
    pub connector_meta: Option<serde_json::Value>,
    pub complete_authorize_url: Option<String>,
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, Clone)]
pub struct CompleteAuthorizeRedirectResponse {
    pub params: Option<Secret<String>>,
    pub payload: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, Default, Clone)]
pub struct PaymentsSyncData {
    //TODO : add fields based on the connector requirements
    pub connector_transaction_id: ResponseId,
    pub encoded_data: Option<String>,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    pub connector_meta: Option<serde_json::Value>,
    pub sync_type: SyncRequestType,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub currency: storage_enums::Currency,
}

#[derive(Debug, Default, Clone)]
pub enum SyncRequestType {
    MultipleCaptureSync(Vec<String>),
    #[default]
    SinglePaymentSync,
}

#[derive(Debug, Default, Clone)]
pub struct PaymentsCancelData {
    pub amount: Option<i64>,
    pub currency: Option<storage_enums::Currency>,
    pub connector_transaction_id: String,
    pub cancellation_reason: Option<String>,
    pub connector_meta: Option<serde_json::Value>,
    pub browser_info: Option<BrowserInformation>,
    pub metadata: Option<pii::SecretSerdeValue>,
    // This metadata is used to store the metadata shared during the payment intent request.
}

#[derive(Debug, Default, Clone)]
pub struct PaymentsRejectData {
    pub amount: Option<i64>,
    pub currency: Option<storage_enums::Currency>,
}

#[derive(Debug, Default, Clone)]
pub struct PaymentsApproveData {
    pub amount: Option<i64>,
    pub currency: Option<storage_enums::Currency>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BrowserInformation {
    pub color_depth: Option<u8>,
    pub java_enabled: Option<bool>,
    pub java_script_enabled: Option<bool>,
    pub language: Option<String>,
    pub screen_height: Option<u32>,
    pub screen_width: Option<u32>,
    pub time_zone: Option<i32>,
    pub ip_address: Option<std::net::IpAddr>,
    pub accept_header: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub enum ResponseId {
    ConnectorTransactionId(String),
    EncodedData(String),
    #[default]
    NoResponseId,
}
impl ResponseId {
    pub fn get_connector_transaction_id(
        &self,
    ) -> errors::CustomResult<String, errors::ValidationError> {
        match self {
            Self::ConnectorTransactionId(txn_id) => Ok(txn_id.to_string()),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "connector_transaction_id",
            })
            .attach_printable("Expected connector transaction ID not found"),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SurchargeDetails {
    /// original_amount
    pub original_amount: common_utils::types::MinorUnit,
    /// surcharge value
    pub surcharge: common_utils::types::Surcharge,
    /// tax on surcharge value
    pub tax_on_surcharge:
        Option<common_utils::types::Percentage<{ consts::SURCHARGE_PERCENTAGE_PRECISION_LENGTH }>>,
    /// surcharge amount for this payment
    pub surcharge_amount: common_utils::types::MinorUnit,
    /// tax on surcharge amount for this payment
    pub tax_on_surcharge_amount: common_utils::types::MinorUnit,
    /// sum of original amount,
    pub final_amount: common_utils::types::MinorUnit,
}

impl SurchargeDetails {
    pub fn is_request_surcharge_matching(
        &self,
        request_surcharge_details: RequestSurchargeDetails,
    ) -> bool {
        request_surcharge_details.surcharge_amount == self.surcharge_amount
            && request_surcharge_details.tax_amount.unwrap_or_default()
                == self.tax_on_surcharge_amount
    }
    pub fn get_total_surcharge_amount(&self) -> common_utils::types::MinorUnit {
        self.surcharge_amount + self.tax_on_surcharge_amount
    }
}

impl
    From<(
        &RequestSurchargeDetails,
        &payments::payment_attempt::PaymentAttempt,
    )> for SurchargeDetails
{
    fn from(
        (request_surcharge_details, payment_attempt): (
            &RequestSurchargeDetails,
            &payments::payment_attempt::PaymentAttempt,
        ),
    ) -> Self {
        let surcharge_amount = request_surcharge_details.surcharge_amount;
        let tax_on_surcharge_amount = request_surcharge_details.tax_amount.unwrap_or_default();
        Self {
            original_amount: payment_attempt.amount,
            surcharge: common_utils::types::Surcharge::Fixed(
                request_surcharge_details.surcharge_amount,
            ),
            tax_on_surcharge: None,
            surcharge_amount,
            tax_on_surcharge_amount,
            final_amount: payment_attempt.amount + surcharge_amount + tax_on_surcharge_amount,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticationData {
    pub eci: Option<String>,
    pub cavv: String,
    pub threeds_server_transaction_id: String,
    pub message_version: String,
}

#[derive(Debug, Clone)]
pub struct RefundsData {
    pub refund_id: String,
    pub connector_transaction_id: String,

    pub connector_refund_id: Option<String>,
    pub currency: storage_enums::Currency,
    /// Amount for the payment against which this refund is issued
    pub payment_amount: i64,
    pub reason: Option<String>,
    pub webhook_url: Option<String>,
    /// Amount to be refunded
    pub refund_amount: i64,
    /// Arbitrary metadata required for refund
    pub connector_metadata: Option<serde_json::Value>,
    pub browser_info: Option<BrowserInformation>,
}

#[derive(Debug, Clone)]
pub struct AccessTokenRequestData {
    pub app_id: Secret<String>,
    pub id: Option<Secret<String>>,
    // Add more keys if required
}

impl TryFrom<router_data::ConnectorAuthType> for AccessTokenRequestData {
    type Error = api_error_response::ApiErrorResponse;
    fn try_from(connector_auth: router_data::ConnectorAuthType) -> Result<Self, Self::Error> {
        match connector_auth {
            router_data::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                app_id: api_key,
                id: None,
            }),
            router_data::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
            }),
            router_data::ConnectorAuthType::SignatureKey { api_key, key1, .. } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
            }),
            router_data::ConnectorAuthType::MultiAuthKey { api_key, key1, .. } => Ok(Self {
                app_id: api_key,
                id: Some(key1),
            }),

            _ => Err(api_error_response::ApiErrorResponse::InvalidDataValue {
                field_name: "connector_account_details",
            }),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AcceptDisputeRequestData {
    pub dispute_id: String,
    pub connector_dispute_id: String,
}

#[derive(Default, Debug, Clone)]
pub struct DefendDisputeRequestData {
    pub dispute_id: String,
    pub connector_dispute_id: String,
}

#[derive(Default, Debug, Clone)]
pub struct SubmitEvidenceRequestData {
    pub dispute_id: String,
    pub connector_dispute_id: String,
    pub access_activity_log: Option<String>,
    pub billing_address: Option<String>,
    pub cancellation_policy: Option<Vec<u8>>,
    pub cancellation_policy_provider_file_id: Option<String>,
    pub cancellation_policy_disclosure: Option<String>,
    pub cancellation_rebuttal: Option<String>,
    pub customer_communication: Option<Vec<u8>>,
    pub customer_communication_provider_file_id: Option<String>,
    pub customer_email_address: Option<String>,
    pub customer_name: Option<String>,
    pub customer_purchase_ip: Option<String>,
    pub customer_signature: Option<Vec<u8>>,
    pub customer_signature_provider_file_id: Option<String>,
    pub product_description: Option<String>,
    pub receipt: Option<Vec<u8>>,
    pub receipt_provider_file_id: Option<String>,
    pub refund_policy: Option<Vec<u8>>,
    pub refund_policy_provider_file_id: Option<String>,
    pub refund_policy_disclosure: Option<String>,
    pub refund_refusal_explanation: Option<String>,
    pub service_date: Option<String>,
    pub service_documentation: Option<Vec<u8>>,
    pub service_documentation_provider_file_id: Option<String>,
    pub shipping_address: Option<String>,
    pub shipping_carrier: Option<String>,
    pub shipping_date: Option<String>,
    pub shipping_documentation: Option<Vec<u8>>,
    pub shipping_documentation_provider_file_id: Option<String>,
    pub shipping_tracking_number: Option<String>,
    pub invoice_showing_distinct_transactions: Option<Vec<u8>>,
    pub invoice_showing_distinct_transactions_provider_file_id: Option<String>,
    pub recurring_transaction_agreement: Option<Vec<u8>>,
    pub recurring_transaction_agreement_provider_file_id: Option<String>,
    pub uncategorized_file: Option<Vec<u8>>,
    pub uncategorized_file_provider_file_id: Option<String>,
    pub uncategorized_text: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RetrieveFileRequestData {
    pub provider_file_id: String,
}

#[serde_as]
#[derive(Clone, Debug, serde::Serialize)]
pub struct UploadFileRequestData {
    pub file_key: String,
    #[serde(skip)]
    pub file: Vec<u8>,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub file_type: mime::Mime,
    pub file_size: i32,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone)]
pub struct PayoutsData {
    pub payout_id: String,
    pub amount: i64,
    pub connector_payout_id: Option<String>,
    pub destination_currency: storage_enums::Currency,
    pub source_currency: storage_enums::Currency,
    pub payout_type: storage_enums::PayoutType,
    pub entity_type: storage_enums::PayoutEntityType,
    pub customer_details: Option<CustomerDetails>,
    pub vendor_details: Option<api_models::payouts::PayoutVendorAccountDetails>,
}

#[derive(Debug, Default, Clone)]
pub struct CustomerDetails {
    pub customer_id: Option<String>,
    pub name: Option<Secret<String, masking::WithType>>,
    pub email: Option<pii::Email>,
    pub phone: Option<Secret<String, masking::WithType>>,
    pub phone_country_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerifyWebhookSourceRequestData {
    pub webhook_headers: actix_web::http::header::HeaderMap,
    pub webhook_body: Vec<u8>,
    pub merchant_secret: api_models::webhooks::ConnectorWebhookSecrets,
}
