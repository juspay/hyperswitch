use cards::CardNumber;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::PaymentsAuthorizeRequestData,
    core::errors,
    pii::Secret,
    types::{self, api, storage::enums},
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
pub struct IntuitAuthUpdateRequest {
    grant_type: IntuitAuthGrantTypes,
    refresh_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntuitAuthGrantTypes {
    RefreshToken,
}

impl TryFrom<&types::RefreshTokenRouterData> for IntuitAuthUpdateRequest {
    type Error = Error;

    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth_type = IntuitAuthType::try_from(&item.connector_auth_type)?;
        let refresh_token = match &item.request.old_access_token {
            None => auth_type.refresh_token,
            Some(old_access_token) => old_access_token
                .refresh_token
                .clone()
                .ok_or(errors::ConnectorError::FailedToObtainAuthType)?,
        };
        Ok(Self {
            grant_type: IntuitAuthGrantTypes::RefreshToken,
            refresh_token,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct IntuitAuthUpdateResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub x_refresh_token_expires_in: i64,
}

pub struct IntuitAuthType {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
}

impl TryFrom<&types::ConnectorAuthType> for IntuitAuthType {
    type Error = Error;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                client_id: key1.to_owned(),
                client_secret: api_key.to_owned(),
                refresh_token: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, IntuitAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<F, IntuitAuthUpdateResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
                refresh_token: Some(item.response.refresh_token),
                refresh_token_created_at: Some(time::OffsetDateTime::now_utc().unix_timestamp()),
                refresh_token_expires: Some(item.response.x_refresh_token_expires_in),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct IntuitPaymentsRequest {
    amount: String,
    currency: enums::Currency,
    description: Option<String>,
    context: IntuitPaymentsRequestContext,
    card: Card,
    capture: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: CardNumber,
    exp_month: Secret<String>,
    exp_year: Secret<String>,
    cvc: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntuitPaymentsRequestContext {
    //Flag that indicates if the charge was made from a mobile device.
    mobile: bool,
    //Flag that indicates if the charge was made for Ecommerce over Web.
    is_ecommerce: bool,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for IntuitPaymentsRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethodData::Card(ref ccard) => {
                let submit_for_settlement = item.request.is_auto_capture()?;
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

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntuitPaymentsCaptureRequest {
    amount: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for IntuitPaymentsCaptureRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount_to_capture.to_string(),
        })
    }
}

// PaymentsResponse
#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IntuitPaymentStatus {
    Authorized,
    Declined,
    Captured,
    Cancelled,
    Settled,
    Refunded,
}

impl From<IntuitPaymentStatus> for enums::AttemptStatus {
    fn from(item: IntuitPaymentStatus) -> Self {
        match item {
            IntuitPaymentStatus::Captured
            | IntuitPaymentStatus::Settled
            | IntuitPaymentStatus::Refunded => Self::Charged,
            IntuitPaymentStatus::Declined => Self::Failure,
            IntuitPaymentStatus::Cancelled => Self::Voided,
            IntuitPaymentStatus::Authorized => Self::Authorized,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum IntuitVoidStatus {
    Issued,
    Declined,
}

impl From<IntuitVoidStatus> for enums::AttemptStatus {
    fn from(item: IntuitVoidStatus) -> Self {
        match item {
            IntuitVoidStatus::Issued => Self::Voided,
            IntuitVoidStatus::Declined => Self::VoidFailed,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct IntuitPaymentsResponse {
    status: IntuitPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, IntuitPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
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
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct IntuitVoidResponse {
    status: IntuitVoidStatus,
    id: String,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, IntuitVoidResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<F, IntuitVoidResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntuitRefundRequest {
    amount: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for IntuitRefundRequest {
    type Error = Error;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount.to_string(),
        })
    }
}

// Type definition for Refund Response
#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum IntuitRefundStatus {
    #[default]
    Declined,
    Issued,
    Settled,
}

impl From<IntuitRefundStatus> for enums::RefundStatus {
    fn from(item: IntuitRefundStatus) -> Self {
        match item {
            IntuitRefundStatus::Issued => Self::Pending,
            IntuitRefundStatus::Settled => Self::Success,
            IntuitRefundStatus::Declined => Self::Failure,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub amount: String,
    pub status: IntuitRefundStatus,
}

impl<T> TryFrom<types::RefundsResponseRouterData<T, RefundResponse>>
    for types::RefundsRouterData<T>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<T, RefundResponse>,
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

#[derive(Debug, Deserialize)]
pub struct IntuitErrorData {
    pub message: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct IntuitErrorResponse {
    pub errors: Vec<IntuitErrorData>,
}

#[derive(Debug, Deserialize)]
pub struct IntuitAuthErrorResponse {
    pub error: String,
}
