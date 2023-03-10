use serde::{Deserialize, Serialize};

use crate::{
    connector::utils,
    core::errors,
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MolliePaymentsRequest {
    amount: Amount,
    description: String,
    redirect_url: String,
    // webhook_url: Option<String>,
    // method: MolliePaymentMethods,
    // metadata: Option<String>,
    // sequence_type: Option<String>,
    // mandate_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MolliePaymentsCancel {
    id: String,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Amount {
    currency: enums::Currency,
    value: String,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Links {
    href: String,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct MollieLinks {
    #[serde(rename = "self")]
    self_: Links,
    checkout: Links,
    dashboard: Links,
    documentation: Links,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MolliePaymentMethods {
    #[serde(rename = "card")]
    Card(CardDetails),
    EPS,
    #[serde(rename = "iDEAL")]
    Ideal,
    #[serde(rename = "Apple Pay")]
    Giropay,
    Sofort,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CardDetails {
    #[serde(rename = "card.number")]
    pub card_number: String,
    #[serde(rename = "card.holder")]
    pub card_holder: String,
    #[serde(rename = "card.expiryMonth")]
    pub card_expiry_month: String,
    #[serde(rename = "card.expiryYear")]
    pub card_expiry_year: String,
    #[serde(rename = "card.cvv")]
    pub card_cvv: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BankDetails {
    billing_email: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for MolliePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.request.currency,
            value: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
        };
        let description = item
            .description
            .clone()
            .unwrap_or("Description".to_string());
        let redirect_url = utils::RouterData::get_return_url(item)?;
        Ok(MolliePaymentsRequest {
            amount,
            description,
            redirect_url,
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for MolliePaymentsCancel {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            id: item.request.connector_transaction_id.to_owned(),
        })
    }
}

pub struct MollieAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for MollieAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MolliePaymentStatus {
    Open,
    Canceled,
    #[default]
    Pending,
    Authorized,
    Expired,
    Failed,
    Paid,
}

impl From<MolliePaymentStatus> for enums::AttemptStatus {
    fn from(item: MolliePaymentStatus) -> Self {
        match item {
            MolliePaymentStatus::Paid => Self::Charged,
            MolliePaymentStatus::Failed => Self::Failure,
            MolliePaymentStatus::Pending => Self::Authorizing,
            MolliePaymentStatus::Open => Self::Started,
            MolliePaymentStatus::Canceled => Self::Voided,
            MolliePaymentStatus::Authorized => Self::Authorized,
            MolliePaymentStatus::Expired => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct MolliePaymentsResponse {
    resource: String,
    id: String,
    mode: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    amount: Amount,
    description: String,
    method: String,
    metadata: String,
    status: MolliePaymentStatus,
    #[serde(rename = "isCancelable")]
    is_cancelable: bool,
    #[serde(rename = "expiresAt")]
    expires_at: String,
    #[serde(rename = "profileId")]
    profile_id: String,
    #[serde(rename = "sequenceType")]
    sequencetype: String,
    #[serde(rename = "redirectUrl")]
    redirecturl: String,
    #[serde(rename = "settlementAmount")]
    settlementamount: Amount,
    #[serde(rename = "_links")]
    links: MollieLinks,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, MolliePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, MolliePaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
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
pub struct MollieRefundRequest {
    amount: Amount,
    id: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for MollieRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.request.currency,
            value: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
        };
        let id = item.request.connector_transaction_id.to_owned();
        Ok(Self { amount, id })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum MollieRefundStatus {
    Queued,
    #[default]
    Pending,
    Processing,
    Refunded,
    Failed,
    Canceled,
}

impl From<MollieRefundStatus> for enums::RefundStatus {
    fn from(item: MollieRefundStatus) -> Self {
        match item {
            MollieRefundStatus::Queued
            | MollieRefundStatus::Pending
            | MollieRefundStatus::Processing => Self::Pending,
            MollieRefundStatus::Refunded => Self::Success,
            MollieRefundStatus::Failed | MollieRefundStatus::Canceled => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    resource: String,
    id: String,
    amount: Amount,
    status: MollieRefundStatus,
    #[serde(rename = "createdAt")]
    created_at: String,
    description: String,
    metadata: String,
    #[serde(rename = "paymentId")]
    payment_id: String,
    #[serde(rename = "_links")]
    links: MollieLinks,
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
                connector_refund_id: item.response.id,
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
        _item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented(
            "try_from RefundsResponseRouterData".to_string(),
        )
        .into())
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct MollieErrorResponse {
    pub status: i32,
    pub title: String,
    pub detail: String,
    pub field: Option<String>,
    #[serde(rename = "_links")]
    pub links: Links,
}
