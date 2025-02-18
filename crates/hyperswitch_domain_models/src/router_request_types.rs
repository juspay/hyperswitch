pub mod authentication;
pub mod fraud_check;
pub mod unified_authentication_service;
use api_models::payments::{AdditionalPaymentData, RequestSurchargeDetails};
use common_utils::{consts, errors, ext_traits::OptionExt, id_type, pii, types::MinorUnit};
use diesel_models::{enums as storage_enums, types::OrderDetailsWithAmount};
use error_stack::ResultExt;
use masking::Secret;
use serde::Serialize;
use serde_with::serde_as;

use super::payment_method_data::PaymentMethodData;
use crate::{
    address,
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
    /// ```text
    /// get_original_amount()
    /// get_surcharge_amount()
    /// get_tax_on_surcharge_amount()
    /// get_total_surcharge_amount() // returns surcharge_amount + tax_on_surcharge_amount
    /// ```
    pub amount: i64,
    pub order_tax_amount: Option<MinorUnit>,
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
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub order_category: Option<String>,
    pub session_token: Option<String>,
    pub enrolled_for_3ds: bool,
    pub related_transaction_id: Option<String>,
    pub payment_experience: Option<storage_enums::PaymentExperience>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub surcharge_details: Option<SurchargeDetails>,
    pub customer_id: Option<id_type::CustomerId>,
    pub request_incremental_authorization: bool,
    pub metadata: Option<serde_json::Value>,
    pub authentication_data: Option<AuthenticationData>,
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,

    // New amount for amount frame work
    pub minor_amount: MinorUnit,

    /// Merchant's identifier for the payment/invoice. This will be sent to the connector
    /// if the connector provides support to accept multiple reference ids.
    /// In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    pub merchant_order_reference_id: Option<String>,
    pub integrity_object: Option<AuthoriseIntegrityObject>,
    pub shipping_cost: Option<MinorUnit>,
    pub additional_payment_method_data: Option<AdditionalPaymentData>,
}

#[derive(Debug, Clone)]
pub struct PaymentsPostSessionTokensData {
    // amount here would include amount, surcharge_amount and shipping_cost
    pub amount: MinorUnit,
    /// original amount sent by the merchant
    pub order_amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub capture_method: Option<storage_enums::CaptureMethod>,
    /// Merchant's identifier for the payment/invoice. This will be sent to the connector
    /// if the connector provides support to accept multiple reference ids.
    /// In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    pub merchant_order_reference_id: Option<String>,
    pub shipping_cost: Option<MinorUnit>,
    pub setup_future_usage: Option<storage_enums::FutureUsage>,
    pub router_return_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthoriseIntegrityObject {
    /// Authorise amount
    pub amount: MinorUnit,
    /// Authorise currency
    pub currency: storage_enums::Currency,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncIntegrityObject {
    /// Sync amount
    pub amount: Option<MinorUnit>,
    /// Sync currency
    pub currency: Option<storage_enums::Currency>,
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
    pub metadata: Option<serde_json::Value>,
    // This metadata is used to store the metadata shared during the payment intent request.
    pub capture_method: Option<storage_enums::CaptureMethod>,

    // New amount for amount frame work
    pub minor_payment_amount: MinorUnit,
    pub minor_amount_to_capture: MinorUnit,
    pub integrity_object: Option<CaptureIntegrityObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureIntegrityObject {
    /// capture amount
    pub capture_amount: Option<MinorUnit>,
    /// capture currency
    pub currency: storage_enums::Currency,
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
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
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
    pub metadata: Option<Secret<serde_json::Value>>,

    // New amount for amount frame work
    pub minor_amount: Option<MinorUnit>,
}

impl TryFrom<PaymentsAuthorizeData> for PaymentsPreProcessingData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: Some(data.payment_method_data),
            amount: Some(data.amount),
            minor_amount: Some(data.minor_amount),
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
            metadata: data.metadata.map(Secret::new),
        })
    }
}

impl TryFrom<CompleteAuthorizeData> for PaymentsPreProcessingData {
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(data: CompleteAuthorizeData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.payment_method_data,
            amount: Some(data.amount),
            minor_amount: Some(data.minor_amount),
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
            metadata: data.connector_meta.map(Secret::new),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PaymentsPostProcessingData {
    pub payment_method_data: PaymentMethodData,
    pub customer_id: Option<id_type::CustomerId>,
    pub connector_transaction_id: Option<String>,
    pub country: Option<common_enums::CountryAlpha2>,
    pub connector_meta_data: Option<pii::SecretSerdeValue>,
    pub header_payload: Option<payments::HeaderPayload>,
}

impl<F> TryFrom<RouterData<F, PaymentsAuthorizeData, response_types::PaymentsResponseData>>
    for PaymentsPostProcessingData
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn try_from(
        data: RouterData<F, PaymentsAuthorizeData, response_types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: data.request.payment_method_data,
            connector_transaction_id: match data.response {
                Ok(response_types::PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(id),
                    ..
                }) => Some(id.clone()),
                _ => None,
            },
            customer_id: data.request.customer_id,
            country: data
                .address
                .get_payment_billing()
                .and_then(|bl| bl.address.as_ref())
                .and_then(|address| address.country),
            connector_meta_data: data.connector_meta_data.clone(),
            header_payload: data.header_payload,
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
    pub metadata: Option<serde_json::Value>,
    pub customer_acceptance: Option<mandates::CustomerAcceptance>,
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
    pub split_payments: Option<common_types::payments::SplitPaymentsRequest>,
    pub amount: MinorUnit,
    pub integrity_object: Option<SyncIntegrityObject>,
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
    pub metadata: Option<serde_json::Value>,
    // This metadata is used to store the metadata shared during the payment intent request.

    // minor amount data for amount framework
    pub minor_amount: Option<MinorUnit>,
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
    pub os_type: Option<String>,
    pub os_version: Option<String>,
    pub device_model: Option<String>,
    pub accept_language: Option<String>,
}

#[cfg(feature = "v2")]
impl From<common_utils::types::BrowserInformation> for BrowserInformation {
    fn from(value: common_utils::types::BrowserInformation) -> Self {
        Self {
            color_depth: value.color_depth,
            java_enabled: value.java_enabled,
            java_script_enabled: value.java_script_enabled,
            language: value.language,
            screen_height: value.screen_height,
            screen_width: value.screen_width,
            time_zone: value.time_zone,
            ip_address: value.ip_address,
            accept_header: value.accept_header,
            user_agent: value.user_agent,
            os_type: value.os_type,
            os_version: value.os_version,
            device_model: value.device_model,
            accept_language: value.accept_language,
        }
    }
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
}

impl SurchargeDetails {
    pub fn get_total_surcharge_amount(&self) -> MinorUnit {
        self.surcharge_amount + self.tax_on_surcharge_amount
    }
}

#[cfg(feature = "v1")]
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
            original_amount: payment_attempt.net_amount.get_order_amount(),
            surcharge: common_utils::types::Surcharge::Fixed(
                request_surcharge_details.surcharge_amount,
            ),
            tax_on_surcharge: None,
            surcharge_amount,
            tax_on_surcharge_amount,
        }
    }
}

#[cfg(feature = "v2")]
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
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticationData {
    pub eci: Option<String>,
    pub cavv: String,
    pub threeds_server_transaction_id: String,
    pub message_version: common_utils::types::SemanticVersion,
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
    pub split_refunds: Option<SplitRefundsRequest>,

    // New amount for amount frame work
    pub minor_payment_amount: MinorUnit,
    pub minor_refund_amount: MinorUnit,
    pub integrity_object: Option<RefundIntegrityObject>,
    pub refund_status: storage_enums::RefundStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefundIntegrityObject {
    /// refund currency
    pub currency: storage_enums::Currency,
    /// refund amount
    pub refund_amount: MinorUnit,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub enum SplitRefundsRequest {
    StripeSplitRefund(StripeSplitRefund),
    AdyenSplitRefund(common_types::domain::AdyenSplitData),
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct StripeSplitRefund {
    pub charge_id: String,
    pub transfer_account_id: String,
    pub charge_type: api_models::enums::PaymentChargeType,
    pub options: ChargeRefundsOptions,
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
    //cancellation policy
    pub cancellation_policy: Option<Vec<u8>>,
    pub cancellation_policy_file_type: Option<String>,
    pub cancellation_policy_provider_file_id: Option<String>,
    pub cancellation_policy_disclosure: Option<String>,
    pub cancellation_rebuttal: Option<String>,
    //customer communication
    pub customer_communication: Option<Vec<u8>>,
    pub customer_communication_file_type: Option<String>,
    pub customer_communication_provider_file_id: Option<String>,
    pub customer_email_address: Option<String>,
    pub customer_name: Option<String>,
    pub customer_purchase_ip: Option<String>,
    //customer signature
    pub customer_signature: Option<Vec<u8>>,
    pub customer_signature_file_type: Option<String>,
    pub customer_signature_provider_file_id: Option<String>,
    //product description
    pub product_description: Option<String>,
    //receipts
    pub receipt: Option<Vec<u8>>,
    pub receipt_file_type: Option<String>,
    pub receipt_provider_file_id: Option<String>,
    //refund policy
    pub refund_policy: Option<Vec<u8>>,
    pub refund_policy_file_type: Option<String>,
    pub refund_policy_provider_file_id: Option<String>,
    pub refund_policy_disclosure: Option<String>,
    pub refund_refusal_explanation: Option<String>,
    //service docs
    pub service_date: Option<String>,
    pub service_documentation: Option<Vec<u8>>,
    pub service_documentation_file_type: Option<String>,
    pub service_documentation_provider_file_id: Option<String>,
    //shipping details docs
    pub shipping_address: Option<String>,
    pub shipping_carrier: Option<String>,
    pub shipping_date: Option<String>,
    pub shipping_documentation: Option<Vec<u8>>,
    pub shipping_documentation_file_type: Option<String>,
    pub shipping_documentation_provider_file_id: Option<String>,
    pub shipping_tracking_number: Option<String>,
    //invoice details
    pub invoice_showing_distinct_transactions: Option<Vec<u8>>,
    pub invoice_showing_distinct_transactions_file_type: Option<String>,
    pub invoice_showing_distinct_transactions_provider_file_id: Option<String>,
    //subscription details
    pub recurring_transaction_agreement: Option<Vec<u8>>,
    pub recurring_transaction_agreement_file_type: Option<String>,
    pub recurring_transaction_agreement_provider_file_id: Option<String>,
    //uncategorized details
    pub uncategorized_file: Option<Vec<u8>>,
    pub uncategorized_file_type: Option<String>,
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
    pub payout_type: Option<storage_enums::PayoutType>,
    pub entity_type: storage_enums::PayoutEntityType,
    pub customer_details: Option<CustomerDetails>,
    pub vendor_details: Option<api_models::payouts::PayoutVendorAccountDetails>,

    // New minor amount for amount framework
    pub minor_amount: MinorUnit,
    pub priority: Option<storage_enums::PayoutSendPriority>,
    pub connector_transfer_method_id: Option<String>,
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
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub email: Option<pii::Email>,
    // Minor Unit amount for amount frame work
    pub minor_amount: MinorUnit,
    pub apple_pay_recurring_details: Option<api_models::payments::ApplePayRecurringPaymentRequest>,
}

#[derive(Debug, Clone, Default)]
pub struct PaymentsTaxCalculationData {
    pub amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub shipping_cost: Option<MinorUnit>,
    pub order_details: Option<Vec<OrderDetailsWithAmount>>,
    pub shipping_address: address::Address,
}

#[derive(Debug, Clone, Default)]
pub struct SdkPaymentsSessionUpdateData {
    pub order_tax_amount: MinorUnit,
    // amount here would include amount, surcharge_amount, order_tax_amount and shipping_cost
    pub amount: MinorUnit,
    /// original amount sent by the merchant
    pub order_amount: MinorUnit,
    pub currency: storage_enums::Currency,
    pub session_id: Option<String>,
    pub shipping_cost: Option<MinorUnit>,
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
    pub webhook_url: Option<String>,
    pub browser_info: Option<BrowserInformation>,
    pub email: Option<pii::Email>,
    pub customer_name: Option<Secret<String>>,
    pub return_url: Option<String>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub request_incremental_authorization: bool,
    pub metadata: Option<pii::SecretSerdeValue>,

    // MinorUnit for amount framework
    pub minor_amount: Option<MinorUnit>,
    pub shipping_cost: Option<MinorUnit>,
}
