use serde::{Deserialize, Serialize};
use masking::Secret;
use crate::{connector::utils::{PaymentsAuthorizeRequestData},core::errors,types::{self,api, storage::enums}};

pub struct StancerRouterData<T> {
    pub amount: i32,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i32,
        T,
    )> for StancerRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i32,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct StancerPaymentsRequest {
    amount: i32,
    currency: String,
    auth: Option<StancerAuth>,
    card: Option<StancerCard>,
    sepa: Option<StancerSepa>,
    customer: Option<StancerCustomer>,
    capture: Option<bool>,
    description: Option<String>,
    order_id: Option<String>,
    unique_id: Option<String>,
    return_url: Option<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct StancerAuth {
    return_url: Option<String>,
    device: Option<String>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct StancerCard {
    name: Secret<String>,
    number: cards::CardNumber,
    exp_month: Secret<i32>,
    exp_year: Secret<i32>,
    cvc: Secret<String>,
    external_id: Option<String>,
    zip_code: Option<String>,
    customer: Option<StancerCustomer>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct StancerSepa {
    name: Secret<String>,
    bic: Option<String>,
    iban: cards::CardNumber,
    mandate: String,
    date_mandate: String,
    customer: Option<StancerCustomer>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct StancerCustomer {
    email: Option<String>,
    name: Option<String>,
    mobile: Option<String>,
    date_birth: Option<i32>,
    legal_id: Option<String>,
    external_id: Option<String>,
}

impl TryFrom<&StancerRouterData<&types::PaymentsAuthorizeRouterData>> for StancerPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &StancerRouterData<&types::PaymentsAuthorizeRouterData>) -> Result<Self,Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card = StancerCard {
                    name: req_card.card_holder_name,
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.to_owned(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct StancerAuthType {
    pub(super) api_key: Secret<String>
}

impl TryFrom<&types::ConnectorAuthType> for StancerAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StancerPaymentStatus {
    Refused,
    Canceled,
    Authorized,
    Capture_Sent,
    Captured,
    Disputed,
    Failed,
    Expired,
    #[default]
    To_Capture,
}

impl From<StancerPaymentStatus> for enums::AttemptStatus {
    fn from(item: StancerPaymentStatus) -> Self {
        match item {
            StancerPaymentStatus::Failed => Self::Failure,
            StancerPaymentStatus::Refused => Self::Failure,
            StancerPaymentStatus::Expired => Self::Failure,
            StancerPaymentStatus::Canceled => Self::Failure,
            StancerPaymentStatus::Disputed => Self::Failure,
            StancerPaymentStatus::Authorized => Self::Authorizing,
            StancerPaymentStatus::To_Capture => Self::Authorizing,
            StancerPaymentStatus::Capture_Sent => Self::Charged,
            StancerPaymentStatus::Captured => Self::Charged,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StancerPaymentsResponse {
    id: String,
    status: StancerPaymentStatus,
    response: String,
    amount: i32,
    currency: String,
    auth: Option<StancerAuth>,
    card: Option<StancerCard>,
    sepa: Option>StancerSepa>,
    customer: Option<StancerCustomer>,
    capture: Option<bool>,
    description: Option<String>,
    order_id: Option<String>,
    unique_id: Option<String>,
    date_trans: i32,
    date_bank: Option<i32>,
    return_url: Option<String>,
    created: i32,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, StancerPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, StancerPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct StancerRefundRequest {
    pub amount: <Option>i32,
    pub payment: String,
}

impl<F> TryFrom<&StancerRouterData<&types::RefundsRouterData<F>>> for StancerRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &StancerRouterData<&types::RefundsRouterData<F>>) -> Result<Self,Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response
#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Failed,
    NotHonored,
    RefundSent,
    Refunded,
    #[default]
    ToRefund,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Failed => Self::Failure,
            RefundStatus::NotHonored => Self::Failure,
            RefundStatus::ToRefund => Self::Pending,
            RefundStatus::RefundSent => Self::Pending,
            RefundStatus::Succeeded => Self::Success,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus
    amount: i32,
    currency: String,
    date_bank: i32,
    date_refund: i32,
    payment: String,
    created: i32,
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

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
     }
 }

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct StancerErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
}
