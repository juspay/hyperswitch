use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{AccessTokenRequestInfo, PaymentsCaptureRequestData},
    core::errors,
    pii::{self, Secret},
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct IntuitAuthUpdateRequest {
    grant_type: IntuitAuthGrantTypes,
    refresh_token: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IntuitAuthGrantTypes {
    RefreshToken,
}

impl TryFrom<&types::RefreshTokenRouterData> for IntuitAuthUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            grant_type: IntuitAuthGrantTypes::RefreshToken,
            refresh_token: item.get_request_id()?,
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct IntuitAuthUpdateResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: String,
    x_refresh_token_expires_in: i64,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, IntuitAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, IntuitAuthUpdateResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct IntuitPaymentsRequest {
    amount: String,
    currency: enums::Currency,
    description: Option<String>,
    context: IntuitPaymentsRequestContext,
    card: Card,
    capture: bool,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: Secret<String, pii::CardNumber>,
    exp_month: Secret<String>,
    exp_year: Secret<String>,
    cvc: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntuitPaymentsRequestContext {
    //Flag that indicates if the charge was made from a mobile device.
    mobile: bool,
    //Flag that indicates if the charge was made for Ecommerce over Web.
    is_ecommerce: bool,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for IntuitPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethodData::Card(ref ccard) => {
                let submit_for_settlement = matches!(
                    item.request.capture_method,
                    Some(enums::CaptureMethod::Automatic) | None
                );
                Ok(Self {
                    amount: item.request.amount.to_string(),
                    currency: item.request.currency,
                    context: IntuitPaymentsRequestContext {
                        mobile: item.request.browser_info.clone().map_or(true, |_| false),
                        is_ecommerce: false,
                    },
                    card: Card {
                        number: ccard.card_number.clone(),
                        exp_month: ccard.card_exp_month.clone(),
                        exp_year: ccard.card_exp_year.clone(),
                        cvc: ccard.card_cvc.clone(),
                    },
                    capture: submit_for_settlement,
                    description: item.description.clone(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntuitPaymentsCaptureRequest {
    amount: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for IntuitPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.get_amount_to_capture()?.to_string(),
        })
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum IntuitPaymentStatus {
    Captured,
    Failed,
    #[default]
    Authorized,
    Issued,
    Declined,
    Settled,
}

impl From<IntuitPaymentStatus> for enums::AttemptStatus {
    fn from(item: IntuitPaymentStatus) -> Self {
        match item {
            IntuitPaymentStatus::Captured | IntuitPaymentStatus::Settled => Self::Charged,
            IntuitPaymentStatus::Failed => Self::Failure,
            IntuitPaymentStatus::Authorized => Self::Authorized,
            IntuitPaymentStatus::Issued => Self::Voided,
            IntuitPaymentStatus::Declined => Self::VoidFailed,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntuitPaymentsResponse {
    status: IntuitPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, IntuitPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<F, IntuitPaymentsResponse, T, types::PaymentsResponseData>,
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

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntuitRefundRequest {
    amount: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for IntuitRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount.to_string(),
        })
    }
}

// Type definition for Refund Response
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum IntuitRefundStatus {
    Issued,
    #[default]
    Declined,
}

impl From<IntuitRefundStatus> for enums::RefundStatus {
    fn from(item: IntuitRefundStatus) -> Self {
        match item {
            IntuitRefundStatus::Issued => Self::Success,
            IntuitRefundStatus::Declined => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub amount: String,
    pub status: IntuitRefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
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
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct IntuitErrorData {
    pub message: String,
    pub code: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct IntuitErrorResponse {
    pub errors: Vec<IntuitErrorData>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct IntuitAuthErrorResponse {
    pub error: String,
}
