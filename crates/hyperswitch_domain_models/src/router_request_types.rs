pub mod authentication;
pub mod fraud_check;
use api_models::payments::RequestSurchargeDetails;
use common_utils::{
    consts, errors, ext_traits::OptionExt, id_type, pii, types as common_types, types::MinorUnit,
};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use masking::Secret;
use serde::Serialize;
use serde_with::serde_as;

use super::payment_method_data::PaymentMethodData;
use crate::{
    errors::api_error_response::ApiErrorResponse,
    mandates, payments,
    router_data::{self, RouterData},
    router_flow_types as flows, router_response_types as response_types,
};
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
    pub charges: Option<PaymentCharges>,

    // New amount for amount frame work
    pub minor_amount: MinorUnit,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct PaymentCharges {
    pub charge_type: api_models::enums::PaymentChargeType,
    pub fees: i64,
    pub transfer_account_id: String,
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

    // New amount for amount frame work
    pub minor_payment_amount: MinorUnit,
    pub minor_amount_to_capture: MinorUnit,
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

impl TryFrom<SetupMandateRequestData> for ConnectorCustomerData {
    type Error = error_stack::Report<ApiErrorResponse>;
    fn try_from(data: SetupMandateRequestData) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.email,
            payment_method_data: data.payment_method_data,
            description: None,
            phone: None,
            name: None,
            preprocessing_id: None,
        })
    }
}
impl
    TryFrom<
        &RouterData<flows::Authorize, PaymentsAuthorizeData, response_types::PaymentsResponseData>,
    > for ConnectorCustomerData
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(
        data: &RouterData<
            flows::Authorize,
            PaymentsAuthorizeData,
            response_types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            email: data.request.email.clone(),
            payment_method_data: data.request.payment_method_data.clone(),
            description: None,
            phone: None,
            name: data.request.customer_name.clone(),
            preprocessing_id: data.preprocessing_id.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PaymentMethodTokenizationData {
    pub payment_method_data: PaymentMethodData,
    pub browser_info: Option<BrowserInformation>,
    pub currency: storage_enums::Currency,
    pub amount: Option<i64>,
}

impl TryFrom<SetupMandateRequestData> for PaymentMethodTokenizationData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: SetupMandateRequestData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            browser_info: None,
            currency: data.currency,
            amount: data.amount,
        })
    }
}
impl<F> From<&RouterData<F, PaymentsAuthorizeData, response_types::PaymentsResponseData>>
    for PaymentMethodTokenizationData
{
    fn from(
        data: &RouterData<F, PaymentsAuthorizeData, response_types::PaymentsResponseData>,
    ) -> Self {
        Self {
            payment_method_data: data.request.payment_method_data.clone(),
            browser_info: None,
            currency: data.request.currency,
            amount: Some(data.request.amount),
        }
    }
}

impl TryFrom<PaymentsAuthorizeData> for PaymentMethodTokenizationData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            browser_info: data.browser_info,
            currency: data.currency,
            amount: Some(data.amount),
        })
    }
}

impl TryFrom<CompleteAuthorizeData> for PaymentMethodTokenizationData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: CompleteAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data
                .payment_method_data
                .get_required_value("payment_method_data")
                .change_context(ApiErrorResponse::MissingRequiredField {
                    field_name: "payment_method_data",
                })?,
            browser_info: data.browser_info,
            currency: data.currency,
            amount: Some(data.amount),
        })
    }
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
    pub enrolled_for_3ds: bool,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub related_transaction_id: Option<String>,
    pub redirect_response: Option<CompleteAuthorizeRedirectResponse>,
}

impl TryFrom<PaymentsAuthorizeData> for PaymentsPreProcessingData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: Some(data.payment_method_data),
            amount: Some(data.amount),
            email: data.email,
            currency: Some(data.currency),
            payment_method_type: data.payment_method_type,
            setup_mandate_details: data.setup_mandate_details,
            capture_method: data.capture_method,
            order_details: data.order_details,
            router_return_url: data.router_return_url,
            webhook_url: data.webhook_url,
            complete_authorize_url: data.complete_authorize_url,
            browser_info: data.browser_info,
            surcharge_details: data.surcharge_details,
            connector_transaction_id: None,
            mandate_id: data.mandate_id,
            related_transaction_id: data.related_transaction_id,
            redirect_response: None,
            enrolled_for_3ds: data.enrolled_for_3ds,
        })
    }
}

impl TryFrom<CompleteAuthorizeData> for PaymentsPreProcessingData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: CompleteAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            amount: Some(data.amount),
            email: data.email,
            currency: Some(data.currency),
            payment_method_type: None,
            setup_mandate_details: data.setup_mandate_details,
            capture_method: data.capture_method,
            order_details: None,
            router_return_url: None,
            webhook_url: None,
            complete_authorize_url: data.complete_authorize_url,
            browser_info: data.browser_info,
            surcharge_details: None,
            connector_transaction_id: data.connector_transaction_id,
            mandate_id: data.mandate_id,
            related_transaction_id: None,
            redirect_response: data.redirect_response,
            enrolled_for_3ds: true,
        })
    }
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

    // New amount for amount frame work
    pub minor_amount: MinorUnit,
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
    pub payment_experience: Option<common_enums::PaymentExperience>,
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
    pub original_amount: MinorUnit,
    /// surcharge value
    pub surcharge: common_utils::types::Surcharge,
    /// tax on surcharge value
    pub tax_on_surcharge:
        Option<common_utils::types::Percentage<{ consts::SURCHARGE_PERCENTAGE_PRECISION_LENGTH }>>,
    /// surcharge amount for this payment
    pub surcharge_amount: MinorUnit,
    /// tax on surcharge amount for this payment
    pub tax_on_surcharge_amount: MinorUnit,
    /// sum of original amount,
    pub final_amount: MinorUnit,
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
    pub fn get_total_surcharge_amount(&self) -> MinorUnit {
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
    pub message_version: common_types::SemanticVersion,
    pub ds_trans_id: Option<String>,
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
    /// Charges associated with the payment
    pub charges: Option<ChargeRefunds>,

    // New amount for amount frame work
    pub minor_payment_amount: MinorUnit,
    pub minor_refund_amount: MinorUnit,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct ChargeRefunds {
    pub charge_id: String,
    pub transfer_account_id: String,
    pub charge_type: api_models::enums::PaymentChargeType,
    pub options: ChargeRefundsOptions,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum ChargeRefundsOptions {
    Destination(DestinationChargeRefund),
    Direct(DirectChargeRefund),
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DirectChargeRefund {
    pub revert_platform_fee: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DestinationChargeRefund {
    pub revert_platform_fee: bool,
    pub revert_transfer: bool,
}

#[derive(Debug, Clone)]
pub struct AccessTokenRequestData {
    pub app_id: Secret<String>,
    pub id: Option<Secret<String>>,
    // Add more keys if required
}

impl TryFrom<router_data::ConnectorAuthType> for AccessTokenRequestData {
    type Error = ApiErrorResponse;
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

            _ => Err(ApiErrorResponse::InvalidDataValue {
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
    pub priority: Option<storage_enums::PayoutSendPriority>,
}

#[derive(Debug, Default, Clone)]
pub struct CustomerDetails {
    pub customer_id: Option<id_type::CustomerId>,
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

#[derive(Debug, Clone)]
pub struct MandateRevokeRequestData {
    pub mandate_id: String,
    pub connector_mandate_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PaymentsSessionData {
    pub amount: i64,
    pub currency: common_enums::Currency,
    pub country: Option<common_enums::CountryAlpha2>,
    pub surcharge_details: Option<SurchargeDetails>,
    pub order_details: Option<Vec<api_models::payments::OrderDetailsWithAmount>>,
}

#[derive(Debug, Clone)]
pub struct SetupMandateRequestData {
    pub currency: storage_enums::Currency,
    pub payment_method_data: PaymentMethodData,
    pub amount: Option<i64>,
    pub confirm: bool,
    pub statement_descriptor_suffix: Option<String>,
    pub customer_acceptance: Option<mandates::CustomerAcceptance>,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub off_session: Option<bool>,
    pub setup_mandate_details: Option<mandates::MandateData>,
    pub router_return_url: Option<String>,
    pub browser_info: Option<BrowserInformation>,
    pub email: Option<pii::Email>,
    pub customer_name: Option<Secret<String>>,
    pub return_url: Option<String>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub request_incremental_authorization: bool,
    pub metadata: Option<pii::SecretSerdeValue>,
}
