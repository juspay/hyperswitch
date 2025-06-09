use common_enums::enums;
use common_utils::types::{AmountConvertor, StringMinorUnit, StringMinorUnitForConnector};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData, RedirectForm},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{Secret, ExposeInterface, PeekInterface};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, RouterData as RouterDataTrait},
};

pub struct PaymangoRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for PaymangoRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// Payment Intent Request
#[derive(Debug, Serialize)]
pub struct PaymangoPaymentIntentRequest {
    pub data: PaymentIntentData,
}

#[derive(Debug, Serialize)]
pub struct PaymentIntentData {
    pub attributes: PaymentIntentAttributes,
}

#[derive(Debug, Serialize)]
pub struct PaymentIntentAttributes {
    pub amount: i64,
    pub currency: String,
    pub payment_method_allowed: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method_options: Option<PaymentMethodOptions>,
}

#[derive(Debug, Serialize)]
pub struct PaymentMethodOptions {
    pub card: CardOptions,
}

#[derive(Debug, Serialize)]
pub struct CardOptions {
    pub request_three_d_secure: String,
}

// Payment Method Request
#[derive(Debug, Serialize)]
pub struct PaymangoPaymentMethodRequest {
    pub data: PaymangoPaymentMethodData,
}

#[derive(Debug, Serialize)]
pub struct PaymangoPaymentMethodData {
    pub attributes: PaymentMethodAttributes,
}

#[derive(Debug, Serialize)]
pub struct PaymentMethodAttributes {
    #[serde(rename = "type")]
    pub method_type: String,
    pub details: CardDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing: Option<BillingDetails>,
}

#[derive(Debug, Serialize)]
pub struct CardDetails {
    pub card_number: cards::CardNumber,
    pub exp_month: u8,
    pub exp_year: u16,
    pub cvc: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct BillingDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<BillingAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BillingAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

// Attach Payment Method Request
#[derive(Debug, Serialize)]
pub struct PaymangoAttachRequest {
    pub data: AttachData,
}

#[derive(Debug, Serialize)]
pub struct AttachData {
    pub attributes: AttachAttributes,
}

#[derive(Debug, Serialize)]
pub struct AttachAttributes {
    pub payment_method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,
}

// PayMongo uses a two-step payment process:
// 1. Create Payment Intent
// 2. Create Payment Method and attach to Payment Intent
// This transformer creates the Payment Intent request
impl TryFrom<&PaymangoRouterData<&PaymentsAuthorizeRouterData>> for PaymangoPaymentIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaymangoRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        // Convert amount from StringMinorUnit to i64
        let minor_amount = StringMinorUnitForConnector::convert_back(
            &StringMinorUnitForConnector,
            item.amount.clone(),
            item.router_data.request.currency,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let amount = minor_amount.get_amount_as_i64();
        
        // Get currency
        let currency = item.router_data.request.currency.to_string();
        
        // For now, we only support card payments
        let payment_method_allowed = vec!["card".to_string()];
        
        // Extract optional fields
        let description = item.router_data.description.clone();
        let statement_descriptor = item.router_data.request.statement_descriptor_suffix.clone();
        
        // Configure 3DS settings based on whether capture is manual or automatic
        let payment_method_options = Some(PaymentMethodOptions {
            card: CardOptions {
                request_three_d_secure: if item.router_data.request.is_auto_capture()? {
                    "automatic".to_string()
                } else {
                    "any".to_string()
                },
            },
        });
        
        Ok(Self {
            data: PaymentIntentData {
                attributes: PaymentIntentAttributes {
                    amount,
                    currency,
                    payment_method_allowed,
                    description,
                    statement_descriptor,
                    payment_method_options,
                },
            },
        })
    }
}

// Payment Method creation transformer
impl TryFrom<&PaymangoRouterData<&PaymentsAuthorizeRouterData>> for PaymangoPaymentMethodRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PaymangoRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match &item.router_data.request.payment_method_data {
            PaymentMethodData::Card(card_data) => {
                // Extract card details
                let card_details = CardDetails {
                    card_number: card_data.card_number.clone(),
                    exp_month: card_data.card_exp_month.clone().expose().parse::<u8>()
                        .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                    exp_year: card_data.card_exp_year.clone().expose().parse::<u16>()
                        .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                    cvc: card_data.card_cvc.clone(),
                };
                
                // Extract billing details if available
                let billing = item.router_data.get_optional_billing().and_then(|billing| {
                    let address = billing.address.as_ref().map(|addr| BillingAddress {
                        line1: addr.line1.as_ref().map(|s| s.clone().expose()),
                        line2: addr.line2.as_ref().map(|s| s.clone().expose()),
                        city: addr.city.clone(),
                        state: addr.state.as_ref().map(|s| s.clone().expose()),
                        postal_code: addr.zip.as_ref().map(|s| s.clone().expose()),
                        country: addr.country.map(|c| c.to_string()),
                    });
                    
                    // Extract name from address details
                    let name = billing.address.as_ref().and_then(|addr| {
                        match (&addr.first_name, &addr.last_name) {
                            (Some(first), Some(last)) => {
                                Some(format!("{} {}", first.clone().expose(), last.clone().expose()).trim().to_string())
                            }
                            (Some(first), None) => Some(first.clone().expose()),
                            (None, Some(last)) => Some(last.clone().expose()),
                            _ => None,
                        }
                    });
                    
                    Some(BillingDetails {
                        address,
                        email: billing.email.as_ref().map(|email| email.peek().to_string()),
                        name,
                        phone: billing.phone.as_ref().and_then(|phone| {
                            phone.number.as_ref().map(|n| n.clone().expose())
                        }),
                    })
                });
                
                Ok(Self {
                    data: PaymangoPaymentMethodData {
                        attributes: PaymentMethodAttributes {
                            method_type: "card".to_string(),
                            details: card_details,
                            billing,
                        },
                    },
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Attach Payment Method transformer
impl PaymangoAttachRequest {
    pub fn new(payment_method_id: String, return_url: Option<String>) -> Self {
        Self {
            data: AttachData {
                attributes: AttachAttributes {
                    payment_method: payment_method_id,
                    return_url,
                },
            },
        }
    }
}

// Auth Struct
pub struct PaymangoAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PaymangoAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// Payment Intent Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymangoPaymentIntentResponse {
    pub data: PaymentIntentResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntentResponseData {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub attributes: PaymentIntentResponseAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntentResponseAttributes {
    pub amount: i64,
    pub currency: String,
    pub status: PaymangoPaymentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_descriptor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method_used: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<i64>,
}

// Payment Method Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymangoPaymentMethodResponse {
    pub data: PaymentMethodResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodResponseData {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymangoPaymentStatus {
    #[serde(rename = "awaiting_payment_method")]
    AwaitingPaymentMethod,
    #[serde(rename = "awaiting_next_action")]
    AwaitingNextAction,
    #[serde(rename = "processing")]
    Processing,
    #[serde(rename = "awaiting_capture")]
    AwaitingCapture,
    #[serde(rename = "succeeded")]
    Succeeded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "canceled")]
    Canceled,
    #[default]
    #[serde(other)]
    Unknown,
}

impl From<PaymangoPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PaymangoPaymentStatus) -> Self {
        match item {
            PaymangoPaymentStatus::Succeeded => Self::Charged,
            PaymangoPaymentStatus::Failed => Self::Failure,
            PaymangoPaymentStatus::Canceled => Self::Voided,
            PaymangoPaymentStatus::AwaitingCapture => Self::Authorized,
            PaymangoPaymentStatus::AwaitingPaymentMethod 
            | PaymangoPaymentStatus::AwaitingNextAction 
            | PaymangoPaymentStatus::Processing => Self::Authorizing,
            PaymangoPaymentStatus::Unknown => Self::Pending,
        }
    }
}

// Response transformer for Payment Intent Response
impl<F, T> TryFrom<ResponseRouterData<F, PaymangoPaymentIntentResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaymangoPaymentIntentResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        let status = common_enums::AttemptStatus::from(response.data.attributes.status.clone());
        
        // Check if 3DS authentication is required
        let redirection_data = if response.data.attributes.status == PaymangoPaymentStatus::AwaitingNextAction {
            response.data.attributes.client_key.as_ref().map(|client_key| {
                // PayMongo requires frontend SDK integration for 3DS
                // Create a redirect form with the client_key parameter
                RedirectForm::Form {
                    endpoint: "".to_string(),
                    method: common_utils::request::Method::Get,
                    form_fields: std::collections::HashMap::from([
                        ("client_key".to_string(), client_key.clone())
                    ]),
                }
            })
        } else {
            None
        };
        
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response.data.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: response.data.attributes.payment_method_used.clone(),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// Capture Request
#[derive(Debug, Serialize)]
pub struct PaymangoCaptureRequest {
    pub data: CaptureData,
}

#[derive(Debug, Serialize)]
pub struct CaptureData {
    pub attributes: CaptureAttributes,
}

#[derive(Debug, Serialize)]
pub struct CaptureAttributes {
    pub amount: i64,
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct PaymangoRefundRequest {
    pub data: RefundData,
}

#[derive(Debug, Serialize)]
pub struct RefundData {
    pub attributes: RefundAttributes,
}

#[derive(Debug, Serialize)]
pub struct RefundAttributes {
    pub amount: i64,
    pub payment_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl<F> TryFrom<&PaymangoRouterData<&RefundsRouterData<F>>> for PaymangoRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymangoRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            data: RefundData {
                attributes: RefundAttributes {
                    amount: {
                        let minor_amount = StringMinorUnitForConnector::convert_back(
                            &StringMinorUnitForConnector,
                            item.amount.clone(),
                            item.router_data.request.currency,
                        )
                        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                        minor_amount.get_amount_as_i64()
                    },
                    payment_id: item.router_data.request.connector_transaction_id.clone(),
                    reason: item.router_data.request.reason.clone(),
                    notes: None,
                    metadata: None,
                },
            },
        })
    }
}

// Type definition for Refund Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymangoRefundResponse {
    pub data: RefundResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponseData {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub attributes: RefundResponseAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponseAttributes {
    pub amount: i64,
    pub currency: String,
    pub payment_id: String,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub status: RefundStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum RefundStatus {
    #[serde(rename = "succeeded")]
    Succeeded,
    #[serde(rename = "failed")]
    Failed,
    #[default]
    #[serde(rename = "processing")]
    Processing,
    #[serde(rename = "pending")]
    Pending,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing | RefundStatus::Pending => Self::Pending,
        }
    }
}

// Refund response transformers
impl TryFrom<RefundsResponseRouterData<Execute, PaymangoRefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, PaymangoRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.data.id.clone(),
                refund_status: enums::RefundStatus::from(item.response.data.attributes.status.clone()),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, PaymangoRefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, PaymangoRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.data.id.clone(),
                refund_status: enums::RefundStatus::from(item.response.data.attributes.status.clone()),
            }),
            ..item.data
        })
    }
}

// Error Response
#[derive(Debug, Serialize, Deserialize)]
pub struct PaymangoErrorResponse {
    pub errors: Vec<PaymongoError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymongoError {
    pub code: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ErrorSource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorSource {
    pub pointer: String,
    pub attribute: String,
}
