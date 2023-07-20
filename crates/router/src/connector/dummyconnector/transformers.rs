use diesel_models::enums::Currency;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::PaymentsAuthorizeRequestData,
    core::errors,
    services,
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct DummyConnectorPaymentsRequest {
    amount: i64,
    currency: Currency,
    payment_method_data: PaymentMethodData,
    return_url: Option<String>
}

#[derive(Debug, serde::Serialize, Eq, PartialEq)]
pub enum PaymentMethodData {
    Card(DummyConnectorCard),
    Wallet(DummyConnectorWallet),
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct DummyConnectorCard {
    name: Secret<String>,
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

#[derive(Debug, serde::Serialize, Eq, PartialEq)]
pub enum DummyConnectorWallet {
    GooglePay
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for DummyConnectorPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card = DummyConnectorCard {
                    name: req_card.card_holder_name,
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.request.amount,
                    currency: item.request.currency,
                    payment_method_data: PaymentMethodData::Card(card),
                    return_url: item.request.router_return_url.clone(),
                })
            }
            api::PaymentMethodData::Wallet(api_models::payments::WalletData::GooglePayRedirect(_)) => {
                Ok(Self {
                    amount: item.request.amount,
                    currency: item.request.currency,
                    payment_method_data: PaymentMethodData::Wallet(DummyConnectorWallet::GooglePay),
                    return_url: item.request.router_return_url.clone(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct DummyConnectorAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for DummyConnectorAuthType {
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
pub enum DummyConnectorPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<DummyConnectorPaymentStatus> for enums::AttemptStatus {
    fn from(item: DummyConnectorPaymentStatus) -> Self {
        match item {
            DummyConnectorPaymentStatus::Succeeded => Self::Charged,
            DummyConnectorPaymentStatus::Failed => Self::Failure,
            DummyConnectorPaymentStatus::Processing => Self::AuthenticationPending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentsResponse {
    status: DummyConnectorPaymentStatus,
    id: String,
    amount: i64,
    currency: Currency,
    created: String,
    payment_method_type: String,
    next_action: Option<DummyConnectorNextAction>,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .next_action
            .and_then(|redirection_data| redirection_data.get_url())
            .map(|redirection_url| {
                services::RedirectForm::from((redirection_url, services::Method::Get))
            });
        println!("redirection_data: {:#?}", redirection_data);
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DummyConnectorNextAction {
    RedirectToUrl(Url),
}

impl DummyConnectorNextAction {
    fn get_url(&self) -> Option<Url> {
        match self {
            Self::RedirectToUrl(redirect_to_url) => Some(redirect_to_url.to_owned()),
        }
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct DummyConnectorRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for DummyConnectorRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
    currency: Currency,
    created: String,
    payment_amount: i64,
    refund_amount: i64,
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DummyConnectorErrorResponse {
    pub error: ErrorData,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorData {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
