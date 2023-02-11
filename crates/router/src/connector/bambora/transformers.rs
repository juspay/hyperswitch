use masking::PeekInterface;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BamboraPaymentsRequest {
    amount: i64,
    payment_method: String,
    card: Card,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for BamboraPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            ..Default::default()
        })
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BamboraPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let payment_method_data = item.request.payment_method_data.clone();
        let payment_method = match payment_method_data {
            api::PaymentMethod::Card(ref _item) => String::from("card"),
            _ => todo!(),
        };
        let card = match payment_method_data {
            api::PaymentMethod::Card(ref item) => Card {
                name: item.card_holder_name.peek().clone(),
                number: item.card_number.peek().clone(),
                expiry_month: item.card_exp_month.peek().clone(),
                expiry_year: item.card_exp_year.peek().clone(),
                cvd: item.card_cvc.peek().clone(),
                complete: true,
            },
            _ => todo!(),
        };
        Ok(Self {
            amount: item.request.amount,
            payment_method,
            card,
        })
    }
}

// Auth Struct
pub struct BamboraAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for BamboraAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = _auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BamboraPaymentStatus {
    #[default]
    ZERO,
    ONE,
}

impl From<BamboraPaymentStatus> for enums::AttemptStatus {
    fn from(item: BamboraPaymentStatus) -> Self {
        match item {
            BamboraPaymentStatus::ONE => Self::Charged,
            BamboraPaymentStatus::ZERO => Self::Failure,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BamboraPaymentsResponse {
    approved: BamboraPaymentStatus,
    id: String,
    authorizing_merchant_id: String,
    description: String,
    message_id: i64,
    message: String,
    auth_code: String,
    created: String,
    order_number: String,
    risk_score: i64,
    amount: i64,
    payment_method: String,
    merchant_data: String,
    contents: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.approved),
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

//
// // REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct BamboraRefundRequest {
    amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BamboraRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount,
        })
    }
}

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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BamboraErrorResponse {
    pub error: ApiErrorResponse,
}

#[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct ApiErrorResponse {
    pub code: i64,
    pub category: i64,
    pub message: String,
    pub reference: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct Card {
    name: String,
    number: String,
    expiry_month: String,
    expiry_year: String,
    cvd: String,
    complete: bool,
}
