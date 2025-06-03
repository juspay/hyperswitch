use common_enums::enums;
use common_utils::types::FloatMajorUnit;
use cards;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{consts, errors};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
};

// Type definition for router data with amount
pub struct AuthipayRouterData<T> {
    pub amount: FloatMajorUnit, // Amount in major units (e.g., dollars instead of cents)
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for AuthipayRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// Basic request/response structs used across multiple operations

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    total: FloatMajorUnit,
    currency: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExpiryDate {
    month: Secret<String>,
    year: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: cards::CardNumber,
    security_code: Secret<String>,
    expiry_date: ExpiryDate,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethod {
    payment_card: Card,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SplitShipment {
    total_count: i32,
    final_shipment: bool,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayPaymentsRequest {
    request_type: &'static str,
    transaction_amount: Amount,
    payment_method: PaymentMethod,
    split_shipment: Option<SplitShipment>,
    incremental_flag: Option<bool>,
}

impl TryFrom<&AuthipayRouterData<&PaymentsAuthorizeRouterData>> for AuthipayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthipayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let expiry_date = ExpiryDate {
                    month: req_card.card_exp_month.clone(),
                    year: req_card.card_exp_year.clone(),
                };
                
                let card = Card {
                    number: req_card.card_number.clone(),
                    security_code: req_card.card_cvc.clone(),
                    expiry_date,
                };
                
                let payment_method = PaymentMethod {
                    payment_card: card,
                };
                
                let transaction_amount = Amount {
                    total: item.amount.clone(),
                    currency: item.router_data.request.currency.to_string(),
                };
                
                // Making split_shipment None since some merchants aren't set up to support it
                // This avoids error code 10421: "The merchant is not setup to support split shipment"
                let split_shipment = None;
                
                Ok(Self {
                    request_type: "PaymentCardPreAuthTransaction",
                    transaction_amount,
                    payment_method,
                    split_shipment,
                    incremental_flag: Some(false),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct AuthipayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for AuthipayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey { api_key, api_secret, .. } => Ok(Self {
                api_key: api_key.to_owned(),
                api_secret: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// Payment Status enum
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuthipayPaymentStatus {
    AUTHORIZED,
    CAPTURED,
    RETURNED,
    DECLINED,
    FAILED,
    #[default]
    PROCESSING,
}

impl From<AuthipayPaymentStatus> for enums::AttemptStatus {
    fn from(item: AuthipayPaymentStatus) -> Self {
        match item {
            AuthipayPaymentStatus::CAPTURED => Self::Charged,
            AuthipayPaymentStatus::DECLINED | AuthipayPaymentStatus::FAILED => Self::Failure,
            AuthipayPaymentStatus::PROCESSING => Self::Pending,
            AuthipayPaymentStatus::AUTHORIZED => Self::Authorized,
            AuthipayPaymentStatus::RETURNED => Self::Voided,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayPaymentsResponse {
    client_request_id: String,
    api_trace_id: String,
    ipg_transaction_id: String,
    order_id: String,
    transaction_type: String,
    transaction_state: AuthipayPaymentStatus,
    payment_method_details: Option<PaymentMethodDetails>,
    transaction_amount: Amount,
    transaction_time: i64,
    approved_amount: Amount,
    transaction_status: String,
    approval_code: String,
    processor: Processor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodDetails {
    payment_card: PaymentCardDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentCardDetails {
    expiry_date: ExpiryDate,
    bin: String,
    last4: String,
    brand: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Processor {
    response_code: String,
    response_message: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, AuthipayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AuthipayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.transaction_state),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.ipg_transaction_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// Type definition for CaptureRequest
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayCaptureRequest {
    request_type: &'static str,
    transaction_amount: Amount,
}

impl TryFrom<&AuthipayRouterData<&PaymentsCaptureRouterData>> for AuthipayCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(
        item: &AuthipayRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            request_type: "PaymentCardPostAuthTransaction",
            transaction_amount: Amount {
                total: item.amount,
                currency: item.router_data.request.currency.to_string(),
            },
        })
    }
}

// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayRefundRequest {
    request_type: &'static str,
    transaction_amount: Amount,
}

impl<F> TryFrom<&AuthipayRouterData<&RefundsRouterData<F>>> for AuthipayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AuthipayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            request_type: "PaymentCardReturnTransaction",
            transaction_amount: Amount {
                total: item.amount.to_owned(),
                currency: item.router_data.request.currency.to_string(),
            },
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

// Reusing the payments response structure for refunds
// because Authipay uses the same endpoint and response format
pub type RefundResponse = AuthipayPaymentsResponse;

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.ipg_transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(match item.response.transaction_state {
                    AuthipayPaymentStatus::RETURNED => RefundStatus::Succeeded,
                    AuthipayPaymentStatus::FAILED | AuthipayPaymentStatus::DECLINED => RefundStatus::Failed,
                    _ => RefundStatus::Processing,
                }),
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
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.ipg_transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(match item.response.transaction_state {
                    AuthipayPaymentStatus::RETURNED => RefundStatus::Succeeded,
                    AuthipayPaymentStatus::FAILED | AuthipayPaymentStatus::DECLINED => RefundStatus::Failed,
                    _ => RefundStatus::Processing,
                }),
            }),
            ..item.data
        })
    }
}

// Error Response structs
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails {
    pub code: Option<String>,
    pub message: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayErrorResponse {
    pub client_request_id: Option<String>,
    pub response_type: Option<String>,
    pub error: ErrorDetails,
    pub api_trace_id: Option<String>,
}

impl From<&AuthipayErrorResponse> for ErrorResponse {
    fn from(item: &AuthipayErrorResponse) -> Self {
        Self {
            status_code: 0, // This will be overridden by the HTTP status code
            code: item.error.code.clone().unwrap_or_default(),
            message: item.error.message.clone(),
            reason: None,
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        }
    }
}
