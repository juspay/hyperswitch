use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use cards;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct MonexRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for MonexRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct MonexPaymentsRequest {
    amount: StringMinorUnit,
    currency: String,
    card: MonexCard,
    merchant_order_id: String,
    description: Option<String>,
    // Whether to auto-capture the payment or just authorize
    complete: bool,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct MonexCard {
    number: cards::CardNumber,
    exp_month: Secret<String>,
    exp_year: Secret<String>,
    cvc: Secret<String>,
}

impl TryFrom<&MonexRouterData<&PaymentsAuthorizeRouterData>> for MonexPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MonexRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = MonexCard {
                    number: req_card.card_number,
                    exp_month: req_card.card_exp_month,
                    exp_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    currency: item.router_data.request.currency.to_string(),
                    card,
                    merchant_order_id: item.router_data.payment_id.clone(),
                    description: item.router_data.description.clone(),
                    complete: item.router_data.request.is_auto_capture()?,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Types
pub struct MonexAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for MonexAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
                ..
            } => Ok(Self {
                api_key: api_key.to_owned(),
                client_id: key1.to_owned(),
                client_secret: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// OAuth Token Request and Response Types
#[derive(Debug, Serialize)]
pub struct MonexOAuthRequest {
    pub grant_type: String,
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct MonexOAuthResponse {
    pub access_token: Secret<String>,
    pub token_type: String,
    pub expires_in: i64,
}
// PaymentsResponse
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MonexPaymentStatus {
    Authorized,
    Captured,
    Failed,
    Pending,
}

impl Default for MonexPaymentStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl From<MonexPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: MonexPaymentStatus) -> Self {
        match item {
            MonexPaymentStatus::Authorized => Self::Authorized,
            MonexPaymentStatus::Captured => Self::Charged,
            MonexPaymentStatus::Failed => Self::Failure,
            MonexPaymentStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonexPaymentsResponse {
    #[serde(rename = "payment_id")]
    pub id: String,
    pub status: MonexPaymentStatus,
    pub amount: Option<StringMinorUnit>,
    pub currency: Option<String>,
    pub error: Option<MonexPaymentError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonexPaymentError {
    code: String,
    message: String,
    details: Option<String>,
    #[serde(rename = "type")]
    error_type: Option<String>,
    request_id: Option<String>,
}

impl Default for MonexPaymentError {
    fn default() -> Self {
        Self {
            code: "unknown".to_string(),
            message: "Unknown error".to_string(),
            details: None,
            error_type: None,
            request_id: None,
        }
    }
}

impl From<MonexPaymentError> for MonexErrorResponse {
    fn from(error: MonexPaymentError) -> Self {
        Self {
            status_code: 400, // Default status code for payment errors
            code: error.code,
            message: error.message,
            reason: error.details.clone(),
            error_type: error.error_type,
            details: error.details.map(|d| serde_json::Value::String(d)),
            request_id: error.request_id,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, MonexPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MonexPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Handle error response if present
        if let Some(error) = item.response.error.clone() {
            // Convert to appropriate connector error based on error code
            let error_response: MonexErrorResponse = error.into();
            let _error_message = format!("[{}] {}", error_response.code, error_response.message);
            
            // Map specific error codes to generic error types
            return match error_response.code.as_str() {
                "authentication_failed" | "invalid_api_key" => {
                    Err(errors::ConnectorError::FailedToObtainAuthType.into())
                }
                "insufficient_funds" | "card_declined" | "invalid_card" | 
                "invalid_card_number" | "invalid_card_expiry" | "invalid_card_cvc" => {
                    // Handle all payment validation errors
                    Err(errors::ConnectorError::RequestEncodingFailed.into())
                }
                "duplicate_transaction" => {
                    Err(errors::ConnectorError::RequestEncodingFailed.into())
                }
                "transaction_not_found" | "payment_not_found" => {
                    Err(errors::ConnectorError::ResponseDeserializationFailed.into())
                }
                "unauthorized_request" => {
                    Err(errors::ConnectorError::FailedToObtainAuthType.into())
                }
                "validation_error" => {
                    Err(errors::ConnectorError::RequestEncodingFailed.into())
                }
                "server_error" | "internal_server_error" => {
                    Err(errors::ConnectorError::ResponseDeserializationFailed.into())
                }
                _ => {
                    Err(errors::ConnectorError::ResponseHandlingFailed.into())
                }
            };
        }
        
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
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

// Payment Capture Types
#[derive(Default, Debug, Serialize)]
pub struct MonexPaymentsCaptureRequest {
    pub amount: StringMinorUnit,
    // Note: payment_id is included in the URL path, not in the request body
}

impl TryFrom<&MonexRouterData<&PaymentsCaptureRouterData>> for MonexPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MonexRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct MonexRefundRequest {
    pub amount: StringMinorUnit,
    pub reason: Option<String>,
}

impl<F> TryFrom<&MonexRouterData<&RefundsRouterData<F>>> for MonexRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MonexRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
            reason: item.router_data.request.reason.clone(),
        })
    }
}

// Type definition for Refund Response

#[derive(Debug, Serialize, Default, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MonexRefundStatus {
    Refunded,
    Failed,
    #[default]
    Pending,
}

impl From<MonexRefundStatus> for enums::RefundStatus {
    fn from(item: MonexRefundStatus) -> Self {
        match item {
            MonexRefundStatus::Refunded => Self::Success,
            MonexRefundStatus::Failed => Self::Failure,
            MonexRefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MonexRefundResponse {
    #[serde(rename = "refund_id")]
    pub id: String,
    pub status: MonexRefundStatus,
    pub amount: Option<StringMinorUnit>,
    pub currency: Option<String>,
    pub payment_id: Option<String>,
    pub created_at: Option<String>,
    pub error: Option<MonexPaymentError>,
}

impl TryFrom<RefundsResponseRouterData<Execute, MonexRefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, MonexRefundResponse>,
    ) -> Result<Self, Self::Error> {
        // Handle error response if present
        if let Some(error) = item.response.error.clone() {
            // Convert to appropriate connector error based on error code
            let error_response: MonexErrorResponse = error.into();
            let _error_message = format!("[{}] {}", error_response.code, error_response.message);
            
            return match error_response.code.as_str() {
                "authentication_failed" | "invalid_api_key" => {
                    Err(errors::ConnectorError::FailedToObtainAuthType.into())
                }
                "payment_not_found" => {
                    Err(errors::ConnectorError::ResponseDeserializationFailed.into())
                }
                "refund_not_allowed" | "refund_amount_exceeds_payment_amount" | "duplicate_refund" => {
                    Err(errors::ConnectorError::RequestEncodingFailed.into())
                }
                "validation_error" => {
                    Err(errors::ConnectorError::RequestEncodingFailed.into())
                }
                "server_error" | "internal_server_error" => {
                    Err(errors::ConnectorError::ResponseDeserializationFailed.into())
                }
                _ => {
                    Err(errors::ConnectorError::ResponseHandlingFailed.into())
                }
            };
        }

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, MonexRefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, MonexRefundResponse>,
    ) -> Result<Self, Self::Error> {
        // Handle error response if present
        if let Some(error) = item.response.error.clone() {
            // Convert to appropriate connector error based on error code
            let error_response: MonexErrorResponse = error.into();
            let _error_message = format!("[{}] {}", error_response.code, error_response.message);
            
            return match error_response.code.as_str() {
                "authentication_failed" | "invalid_api_key" => {
                    Err(errors::ConnectorError::FailedToObtainAuthType.into())
                }
                "refund_not_found" => {
                    Err(errors::ConnectorError::ResponseDeserializationFailed.into())
                }
                "validation_error" => {
                    Err(errors::ConnectorError::RequestEncodingFailed.into())
                }
                "server_error" | "internal_server_error" => {
                    Err(errors::ConnectorError::ResponseDeserializationFailed.into())
                }
                _ => {
                    Err(errors::ConnectorError::ResponseHandlingFailed.into())
                }
            };
        }
        
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

// Error Response Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MonexErrorType {
    ValidationError,
    AuthenticationError,
    AuthorizationError,
    PaymentError,
    RefundError,
    ApiError,
    RateLimitError,
    ServerError,
    Unknown,
}

impl Default for MonexErrorType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonexErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub details: Option<serde_json::Value>,
    pub request_id: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonexDetailedErrorResponse {
    pub errors: Vec<MonexErrorResponse>,
}
