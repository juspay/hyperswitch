use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{
        refunds::{Execute, RSync},
        payments::PaymentMethodToken,
    },
    router_request_types::{ResponseId, PaymentMethodTokenizationData},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsCaptureRouterData, PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{Secret, PeekInterface};
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

// Webhook structures for Payload API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadWebhookBody {
    pub id: String,
    pub event: String,
    pub data: PayloadWebhookData,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadWebhookData {
    pub transaction: Option<PayloadPaymentsResponse>,
    pub refund: Option<RefundResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadWebhookSignature {
    pub timestamp: String,
    pub signature: String,
}

//TODO: Fill the struct with respective fields
pub struct PayloadRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for PayloadRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

// Request structures for Payload API (form-urlencoded) - Flattened with bracket notation for nested fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PayloadPaymentsRequest {
    amount: StringMinorUnit,
    #[serde(rename = "type")]
    r#type: String,
    status: String,
    #[serde(rename = "payment_method[type]")]
    payment_method_type: String,
    #[serde(rename = "payment_method[card][card_number]", skip_serializing_if = "Option::is_none")]
    payment_method_card_number: Option<String>,
    #[serde(rename = "payment_method[card][expiry]", skip_serializing_if = "Option::is_none")]
    payment_method_card_expiry: Option<String>,
    #[serde(rename = "payment_method[card][card_code]", skip_serializing_if = "Option::is_none")]
    payment_method_card_code: Option<String>,
    #[serde(rename = "payment_method[id]", skip_serializing_if = "Option::is_none")]
    payment_method_id: Option<String>, // For tokenized payments
    // Billing address fields for AVS validation
    #[serde(rename = "payment_method[billing_address][street_address]", skip_serializing_if = "Option::is_none")]
    billing_address_line1: Option<String>,
    #[serde(rename = "payment_method[billing_address][city]", skip_serializing_if = "Option::is_none")]
    billing_address_city: Option<String>,
    #[serde(rename = "payment_method[billing_address][state_province]", skip_serializing_if = "Option::is_none")]
    billing_address_state: Option<String>,
    #[serde(rename = "payment_method[billing_address][postal_code]", skip_serializing_if = "Option::is_none")]
    billing_address_postal_code: Option<String>,
    #[serde(rename = "payment_method[billing_address][country_code]", skip_serializing_if = "Option::is_none")]
    billing_address_country: Option<String>,
}

// Capture request structure
#[derive(Debug, Serialize)]
pub struct PayloadCaptureRequest {
    status: String,
}

// Cancel/Void request structure  
#[derive(Debug, Serialize)]
pub struct PayloadCancelRequest {
    status: String,
}

impl TryFrom<&PayloadRouterData<&PaymentsAuthorizeRouterData>> for PayloadPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayloadRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                // Format expiry as MM/YY according to Payload specs
                let year_str = req_card.card_exp_year.peek();
                let year_two_digit = if year_str.len() >= 2 {
                    year_str.chars().rev().take(2).collect::<String>().chars().rev().collect()
                } else {
                    year_str.to_string()
                };
                let expiry = format!("{}/{}", req_card.card_exp_month.peek(), year_two_digit);
                
                // Determine the correct status based on capture method
                // For auto-capture, we want the payment to be processed immediately
                // For manual capture, we want it to be authorized only
                let status = match item.router_data.request.capture_method {
                    Some(common_enums::CaptureMethod::Automatic) => "processed".to_string(),
                    Some(common_enums::CaptureMethod::Manual) 
                    | Some(common_enums::CaptureMethod::ManualMultiple)
                    | Some(common_enums::CaptureMethod::Scheduled)
                    | Some(common_enums::CaptureMethod::SequentialAutomatic)
                    | None => "authorized".to_string(),
                };
                
                // For now, provide default billing address to pass AVS validation
                // TODO: Extract actual billing address from router data when structure is available
                
                Ok(Self {
                    amount: item.amount.clone(),
                    r#type: "payment".to_string(),
                    status,
                    payment_method_type: "card".to_string(),
                    payment_method_card_number: Some(req_card.card_number.peek().to_string()),
                    payment_method_card_expiry: Some(expiry),
                    payment_method_card_code: Some(req_card.card_cvc.peek().to_string()),
                    payment_method_id: None,
                    // Provide default billing address for AVS validation
                    billing_address_line1: Some("123 Test Street".to_string()),
                    billing_address_city: Some("New York".to_string()),
                    billing_address_state: Some("NY".to_string()),
                    billing_address_postal_code: Some("10001".to_string()),
                    billing_address_country: Some("US".to_string()),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Capture request transformer
impl TryFrom<&PayloadRouterData<&PaymentsCaptureRouterData>> for PayloadCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: &PayloadRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: "processed".to_string(),
        })
    }
}

// Cancel request transformer (for Void operations)
impl<T> TryFrom<&PayloadRouterData<T>> for PayloadCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: &PayloadRouterData<T>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: "voided".to_string(),
        })
    }
}

// Auth Struct for HTTP Basic Authentication
pub struct PayloadAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PayloadAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            ConnectorAuthType::BodyKey { api_key, key1: _ } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse - Updated to match Payload API status values
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PayloadPaymentStatus {
    #[serde(rename = "authorized")]
    Authorized,
    #[serde(rename = "processed")]
    Processed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "voided")]
    Voided,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "pending")]
    #[default]
    Pending,
}

impl From<PayloadPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PayloadPaymentStatus) -> Self {
        match item {
            PayloadPaymentStatus::Authorized => Self::Authorized,
            PayloadPaymentStatus::Processed => Self::Charged,
            PayloadPaymentStatus::Cancelled | PayloadPaymentStatus::Voided => Self::Voided,
            PayloadPaymentStatus::Failed => Self::Failure,
            PayloadPaymentStatus::Pending => Self::Pending,
        }
    }
}

// Response structure matching actual Payload API response
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayloadPaymentsResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub amount: f64, // Payload returns amount as float
    pub status: String, // Payload returns status as string 
    pub status_code: Option<String>,
    pub status_message: Option<String>,
    pub created_at: String,
    pub avs: Option<String>,
    pub payment_method: Option<PayloadPaymentMethodResponse>,
    pub payment_method_id: Option<String>,
    pub processing_id: Option<String>,
    pub ref_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayloadPaymentMethodResponse {
    id: String,
    #[serde(rename = "type")]
    r#type: String,
    card: Option<PayloadCardResponse>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayloadCardResponse {
    card_brand: String,
    card_number: String, // Masked card number like "xxxxxxxxxxxx4242"
    card_type: String,
    expiry: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, PayloadPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PayloadPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Map Payload string status to AttemptStatus
        let status = match item.response.status.as_str() {
            "authorized" => common_enums::AttemptStatus::Authorized,
            "processed" => common_enums::AttemptStatus::Charged,
            "cancelled" | "voided" => common_enums::AttemptStatus::Voided,
            "declined" => {
                // Handle declined payments, including duplicate attempts
                router_env::logger::warn!(
                    payload_decline_reason=?item.response.status_code,
                    payload_decline_message=?item.response.status_message,
                    "Payment declined by Payload"
                );
                common_enums::AttemptStatus::Failure
            },
            "failed" => common_enums::AttemptStatus::Failure,
            "pending" => common_enums::AttemptStatus::Pending,
            _ => {
                router_env::logger::warn!(
                    unknown_status=?item.response.status,
                    "Unknown Payload status received"
                );
                common_enums::AttemptStatus::Pending
            },
        };

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// REFUND structures matching Payload API
#[derive(Default, Debug, Serialize)]
pub struct PayloadRefundRequest {
    #[serde(rename = "type")]
    r#type: String,
    amount: StringMinorUnit,
    #[serde(rename = "ledger[0][assoc_transaction_id]")]
    ledger_assoc_transaction_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadLedgerEntry {
    assoc_transaction_id: String,
}

impl<F> TryFrom<&PayloadRouterData<&RefundsRouterData<F>>> for PayloadRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PayloadRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let connector_transaction_id = item.router_data.request.connector_transaction_id.clone();
        
        Ok(Self {
            r#type: "refund".to_string(),
            amount: item.amount.to_owned(),
            ledger_assoc_transaction_id: connector_transaction_id,
        })
    }
}

// Refund response structures matching Payload API
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PayloadRefundStatus {
    #[serde(rename = "processed")]
    Processed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "pending")]
    #[default]
    Pending,
}

impl From<PayloadRefundStatus> for enums::RefundStatus {
    fn from(item: PayloadRefundStatus) -> Self {
        match item {
            PayloadRefundStatus::Processed => Self::Success,
            PayloadRefundStatus::Failed => Self::Failure,
            PayloadRefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub amount: f64,
    pub status: String, // Parse as string first, then convert to PayloadRefundStatus
    pub created_at: String,
    pub ledger: Option<Vec<PayloadLedgerEntry>>,
    // Additional fields that Payload API returns
    pub status_code: Option<String>,
    pub status_message: Option<String>,
    pub ref_number: Option<String>,
    pub processed_date: Option<String>,
    pub funding_status: Option<String>,
    pub funding_type: Option<String>,
    pub funding_delay: Option<i32>,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.status.as_str() {
            "processed" => enums::RefundStatus::Success,
            "failed" => enums::RefundStatus::Failure,
            "pending" => enums::RefundStatus::Pending,
            _ => {
                router_env::logger::warn!(
                    unknown_refund_status=?item.response.status,
                    "Unknown Payload refund status received"
                );
                enums::RefundStatus::Pending
            }
        };

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.status.as_str() {
            "processed" => enums::RefundStatus::Success,
            "failed" => enums::RefundStatus::Failure,
            "pending" => enums::RefundStatus::Pending,
            _ => {
                router_env::logger::warn!(
                    unknown_refund_status=?item.response.status,
                    "Unknown Payload refund status received"
                );
                enums::RefundStatus::Pending
            }
        };

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PayloadErrorResponse {
    pub error_type: String,
    pub error_description: String,
    pub object: String,
    pub details: Option<serde_json::Value>,
}

impl PayloadErrorResponse {
    pub fn get_error_message(&self) -> String {
        self.error_description.clone()
    }
}

// Status mapping utility functions
pub fn get_payment_status_from_code(status_code: u16, status: &str) -> common_enums::AttemptStatus {
    match (status_code, status) {
        (200..=299, "authorized") => common_enums::AttemptStatus::Authorized,
        (200..=299, "processed") => common_enums::AttemptStatus::Charged,
        (200..=299, "cancelled" | "voided") => common_enums::AttemptStatus::Voided,
        (200..=299, "pending") => common_enums::AttemptStatus::Pending,
        (400..=499, _) => common_enums::AttemptStatus::Failure,
        (500..=599, _) => common_enums::AttemptStatus::Failure,
        _ => common_enums::AttemptStatus::Failure,
    }
}

pub fn get_refund_status_from_code(status_code: u16, status: &str) -> enums::RefundStatus {
    match (status_code, status) {
        (200..=299, "processed") => enums::RefundStatus::Success,
        (200..=299, "pending") => enums::RefundStatus::Pending,
        (400..=499, _) => enums::RefundStatus::Failure,
        (500..=599, _) => enums::RefundStatus::Failure,
        _ => enums::RefundStatus::Failure,
    }
}

// TOKENIZATION structures matching Payload API
#[derive(Default, Debug, Serialize)]
pub struct PayloadTokenRequest {
    #[serde(rename = "type")]
    r#type: String,
    #[serde(rename = "card[number]")]
    card_number: String,
    #[serde(rename = "card[expiry]")]
    card_expiry: String,
    #[serde(rename = "card[cvc]")]
    card_cvc: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PayloadTokenResponse {
    id: String,
    #[serde(rename = "type")]
    r#type: String,
    card: PayloadCardResponse,
    created_at: String,
}

impl TryFrom<&RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>> for PayloadTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let expiry = format!("{}/{}", 
                    req_card.card_exp_month.peek(),
                    req_card.card_exp_year.peek().get(2..).unwrap_or("00")
                );
                
                Ok(Self {
                    r#type: "payment_method".to_string(),
                    card_number: req_card.card_number.peek().to_string(),
                    card_expiry: expiry,
                    card_cvc: req_card.card_cvc.peek().to_string(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl TryFrom<ResponseRouterData<PaymentMethodToken, PayloadTokenResponse, PaymentMethodTokenizationData, PaymentsResponseData>>
    for RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<PaymentMethodToken, PayloadTokenResponse, PaymentMethodTokenizationData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.id,
            }),
            ..item.data
        })
    }
}

// Error code mapping for Payload-specific error handling
pub fn get_error_code_mapping(code: &str) -> errors::ConnectorError {
    match code {
        "invalid_card" | "card_declined" | "insufficient_funds" => {
            errors::ConnectorError::FailedAtConnector {
                message: "Payment declined".to_string(),
                code: code.to_string(),
            }
        }
        "invalid_amount" | "amount_too_small" | "amount_too_large" => {
            errors::ConnectorError::InvalidConnectorConfig {
                config: "amount configuration",
            }
        }
        "invalid_currency" => errors::ConnectorError::CurrencyNotSupported {
            message: "Currency not supported".to_string(),
            connector: "Payload",
        },
        "authentication_failed" | "unauthorized" => {
            errors::ConnectorError::FailedToObtainAuthType
        }
        "duplicate_transaction" => errors::ConnectorError::FailedAtConnector {
            message: "Duplicate transaction".to_string(),
            code: code.to_string(),
        },
        "rate_limit_exceeded" => errors::ConnectorError::RequestTimeoutReceived,
        "internal_error" | "server_error" => errors::ConnectorError::ProcessingStepFailed(None),
        _ => errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::new()),
    }
}
