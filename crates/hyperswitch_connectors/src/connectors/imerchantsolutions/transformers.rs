use common_enums::enums;
use common_utils::{pii, types::MinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

//TODO: Fill the struct with respective fields
pub struct ImerchantsolutionsRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for ImerchantsolutionsRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct ImerchantsolutionsPaymentsRequest {
    amount: MinorUnit,
    card: ImerchantsolutionsCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ImerchantsolutionsCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&ImerchantsolutionsRouterData<&PaymentsAuthorizeRouterData>>
    for ImerchantsolutionsPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ImerchantsolutionsRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(_) => Err(errors::ConnectorError::NotImplemented(
                "Card payment method not implemented".to_string(),
            )
            .into()),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct ImerchantsolutionsAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for ImerchantsolutionsAuthType {
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
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ImerchantsolutionsPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<ImerchantsolutionsPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: ImerchantsolutionsPaymentStatus) -> Self {
        match item {
            ImerchantsolutionsPaymentStatus::Succeeded => Self::Charged,
            ImerchantsolutionsPaymentStatus::Failed => Self::Failure,
            ImerchantsolutionsPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImerchantsolutionsPaymentsResponse {
    status: ImerchantsolutionsPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<ResponseRouterData<F, ImerchantsolutionsPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, ImerchantsolutionsPaymentsResponse, T, PaymentsResponseData>,
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
                authentication_data: None,
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
pub struct ImerchantsolutionsRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&ImerchantsolutionsRouterData<&RefundsRouterData<F>>>
    for ImerchantsolutionsRefundRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ImerchantsolutionsRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ImerchantsolutionsErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImerchantsolutionsWebhookData {
    #[serde(rename = "type")]
    pub event_type: ImerchantsolutionsWebhookEventType,
    pub payment_id: String,
    pub psp_reference: String,
    pub original_reference: Option<String>,
    pub reference: Option<String>,
    pub merchant_reference: Option<String>,
    pub status: ImerchantsolutionsWebhookStatus,
    pub reason: Option<String>,
    pub error: Option<String>,
    amount: Option<MinorUnit>,
    refunded_amount: Option<MinorUnit>,
    total_refunded: Option<MinorUnit>,
    currency: enums::Currency,
    processor: Option<String>,
    card_last4: Option<String>,
    card_brand: Option<String>,
    customer_email: Option<pii::Email>,
    partner_id: Option<Secret<String>>,
    merchant_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImerchantsolutionsWebhookEventType {
    #[serde(rename = "payment.completed")]
    PaymentCompleted,
    #[serde(rename = "payment.cancelled")]
    PaymentCancelled,
    #[serde(rename = "payment.failed")]
    PaymentFailed,
    #[serde(rename = "payment.refunded")]
    PaymentRefunded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImerchantsolutionsWebhookStatus {
    PartiallyCaptured,
    Captured,
    PartiallyRefunded,
    Refunded,
    Cancelled,
    Failed,
    Refused,
}
