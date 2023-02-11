use serde::{Deserialize, Serialize};
use crate::{connector::utils, core::errors,types::{self,api, storage::enums}, pii::PeekInterface};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntuitPaymentsRequest {
    amount: String,
    currency: String,
    description: String,
    context: Context,
    card: Card,
    capture: bool
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: String,
    exp_month: String,
    exp_year: String,
    cvc: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    mobile: bool,
    is_ecommerce: bool,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for IntuitPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let submit_for_settlement = matches!(
                    item.request.capture_method,
                    Some(enums::CaptureMethod::Automatic) | None
                );
                Ok(Self {
                    amount: item.request.amount.to_string(),
                    currency: item.request.currency.to_string().to_uppercase(),
                    context: Context { 
                        mobile: item.request.browser_info.clone().map_or(true, |_| false),
                        is_ecommerce: false
                    },
                    card: Card {
                        number: ccard.card_number.peek().clone(),
                        exp_month: ccard.card_exp_month.peek().clone(),
                        exp_year: ccard.card_exp_year.peek().clone(),
                        cvc: ccard.card_cvc.peek().clone(),
                    },
                    capture: submit_for_settlement,
                    ..Default::default()
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
    fn try_from(value: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: value
                .request
                .amount_to_capture
                .map(|amount| amount.to_string())
                .ok_or_else(utils::missing_field_err("amount_to_capture"))?
        })
    }
}

pub struct IntuitAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for IntuitAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
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
    Declined
}

impl From<IntuitPaymentStatus> for enums::AttemptStatus {
    fn from(item: IntuitPaymentStatus) -> Self {
        match item {
            IntuitPaymentStatus::Captured => Self::Charged,
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

impl<F,T> TryFrom<types::ResponseRouterData<F, IntuitPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, IntuitPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
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
    amount: String
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for IntuitRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount.to_string()
        })
    }
}

// Type definition for Refund Response
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub amount: i64,
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
                refund_status: enums::RefundStatus::default()
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::default()
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct IntuitErrorResponse {
    pub status: Option<String>,
    pub message: String,
}
