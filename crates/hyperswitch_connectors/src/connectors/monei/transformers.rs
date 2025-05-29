use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, AddressData},
};

//TODO: Fill the struct with respective fields
pub struct MoneiRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for MoneiRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct MoneiPaymentsRequest {
    amount: i32, // Changed from StringMinorUnit to i32 as required by Monei
    currency: enums::Currency,
    #[serde(rename = "orderId")]
    order_id: String,
    description: Option<String>,
    #[serde(rename = "paymentMethod")]
    payment_method: MoneiPaymentMethod,
    customer: Option<MoneiCustomer>,
    #[serde(rename = "transactionType")]
    transaction_type: MoneiTransactionType,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MoneiTransactionType {
    Sale,
    Auth,
}

impl Default for MoneiTransactionType {
    fn default() -> Self {
        Self::Sale
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct MoneiPaymentMethod {
    card: MoneiCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct MoneiCard {
    number: cards::CardNumber,
    #[serde(rename = "expMonth")]
    exp_month: Secret<String>,
    #[serde(rename = "expYear")]
    exp_year: Secret<String>,
    cvc: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct MoneiCustomer {
    email: Option<common_utils::pii::Email>,
    name: Option<Secret<String>>,
}

impl TryFrom<&MoneiRouterData<&PaymentsAuthorizeRouterData>> for MoneiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MoneiRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = MoneiCard {
                    number: req_card.card_number,
                    exp_month: req_card.card_exp_month,
                    exp_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                };
                
                let payment_method = MoneiPaymentMethod { card };
                
                let customer = item.router_data.address.get_payment_billing().map(|billing| {
                    MoneiCustomer {
                        email: billing.email.clone(),
                        name: billing.get_optional_full_name(),
                    }
                });
                
                let transaction_type = if item.router_data.request.is_auto_capture()? {
                    MoneiTransactionType::Sale
                } else {
                    MoneiTransactionType::Auth
                };
                
                // Get amount as i64 first, then convert to i32
                let amount_i64 = item.router_data.request.amount;
                let amount = amount_i64.to_string().parse::<i32>()
                    .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;
                
                Ok(Self {
                    amount,
                    currency: item.router_data.request.currency,
                    order_id: item.router_data.connector_request_reference_id.clone(),
                    description: item.router_data.description.clone(),
                    payment_method,
                    customer,
                    transaction_type,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct MoneiAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for MoneiAuthType {
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
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MoneiPaymentStatus {
    Succeeded,
    Failed,
    Pending,
    Authorized,
    Expired,
    Canceled,
    Refunded,
    PartiallyRefunded,
    #[default]
    #[serde(other)]
    Unknown,
}

impl From<MoneiPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: MoneiPaymentStatus) -> Self {
        match item {
            MoneiPaymentStatus::Succeeded => Self::Charged,
            MoneiPaymentStatus::Failed => Self::Failure,
            MoneiPaymentStatus::Pending => Self::Pending,
            MoneiPaymentStatus::Authorized => Self::Authorized,
            MoneiPaymentStatus::Expired => Self::Failure,
            MoneiPaymentStatus::Canceled => Self::Voided,
            MoneiPaymentStatus::Refunded => Self::AutoRefunded,
            MoneiPaymentStatus::PartiallyRefunded => Self::PartialCharged,
            MoneiPaymentStatus::Unknown => Self::Pending,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoneiPaymentsResponse {
    status: MoneiPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, MoneiPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MoneiPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
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

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct MoneiRefundRequest {
    pub amount: i32,
}

impl<F> TryFrom<&MoneiRouterData<&RefundsRouterData<F>>> for MoneiRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MoneiRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        // Get amount as i64 first, then convert to i32
        let amount_i64 = item.router_data.request.refund_amount;
        let amount = amount_i64.to_string().parse::<i32>()
            .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;
        
        Ok(Self {
            amount,
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
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
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

// Error response structure based on Monei API documentation
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoneiErrorResponse {
    pub status: String,
    #[serde(rename = "statusCode")]
    pub status_code: u16,
    pub message: String,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(rename = "requestTime")]
    pub request_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default)]
    pub code: String,
}
