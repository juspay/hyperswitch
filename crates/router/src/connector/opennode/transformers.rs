use std::collections::HashMap;
use std::default;

use crate::{
    connector::utils::PaymentsAuthorizeRequestData,
    core::errors,
    services,
    types::{self, api, storage::enums},
};
use masking::Secret;
use serde::{Deserialize, Serialize};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct OpennodePaymentsRequest {
    amount: i64,
    currency: String,
    description: String,
    ttl: i64,
    auto_settle: bool,
    success_url: String,
    callback_url: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for OpennodePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        get_crypto_specific_payment_data(item)
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct OpennodeAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for OpennodeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OpennodePaymentStatus {
    #[serde(rename = "unpaid")]
    Unpaid,
    #[serde(rename = "paid")]
    Paid,
    #[serde(rename = "expired")]
    Expired,
    #[default]
    #[serde(rename = "processing")]
    Processing,
    #[serde(rename = "underpaid")]
    Underpaid,
    #[serde(rename = "refunded")]
    Refunded,
}

impl From<OpennodePaymentStatus> for enums::AttemptStatus {
    fn from(item: OpennodePaymentStatus) -> Self {
        match item {
            OpennodePaymentStatus::Unpaid => Self::AuthenticationPending,
            OpennodePaymentStatus::Paid => Self::Charged,
            OpennodePaymentStatus::Expired => Self::Failure,
            _ => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpennodePaymentsResponseData {
    id: String,
    hosted_checkout_url: String,
    status: OpennodePaymentStatus,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpennodePaymentsResponse {
    data: OpennodePaymentsResponseData,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, OpennodePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            OpennodePaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let form_fields = HashMap::new();
        let redirection_data = services::RedirectForm {
            endpoint: item.response.data.hosted_checkout_url.to_string(),
            method: services::Method::Get,
            form_fields,
        };
        let attempt_status = item.response.data.status;
        Ok(Self {
            status: enums::AttemptStatus::from(attempt_status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.data.id),
                redirection_data: Some(redirection_data),
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct OpennodeRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for OpennodeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount,
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

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct OpennodeErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

fn get_crypto_specific_payment_data<'a>(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<OpennodePaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let amount = item.request.amount;
    let currency = item.request.currency.to_string();
    let description = item.description.as_ref().unwrap().to_string();
    let ttl = 10i64;
    let auto_settle = true;
    let success_url = item.return_url.as_ref().unwrap().to_string();
    let callback_url = "https://enqomrzd84n3c.x.pipedream.net/".to_string();

    Ok(OpennodePaymentsRequest {
        amount,
        currency,
        description,
        ttl,
        auto_settle,
        success_url,
        callback_url,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpennodeWebhookDetails {
    pub id: String,
    pub callback_url: String,
    pub success_url: String,
    pub status: OpennodePaymentStatus,
    pub payment_method: String,
    pub missing_amt: String,
    pub order_id: String,
    pub description: String,
    pub price: String,
    pub fee: String,
    pub auto_settle: String,
    pub fiat_value: String,
    pub net_fiat_value: String,
    pub overpaid_by: String,
    pub hashed_order: String,
}
