use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{core::errors,types::{self,api, storage::enums}};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AmountMoneyType {
    amount: i64,
    currency: String
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct SquarePaymentsRequest {
    amount_money: AmountMoneyType,
    idempotency_key: String,
    source_id: String,
}

// i am using statement_descriptor_suffix as a means to supply card token to the request, as create-payment on square seems to use tokens instead of payment options
impl TryFrom<&types::PaymentsAuthorizeRouterData> for SquarePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let payment_request = Self {
            amount_money: AmountMoneyType {
                amount: item.request.amount,
                currency: item.request.currency.to_string()
            },
            idempotency_key: Uuid::new_v4().to_string(),
            source_id: match &item.request.statement_descriptor_suffix {
                Some (val) => String::from(val),
                None => String::from("cnon:card-nonce-ok")
            }
        };
        Ok(payment_request)
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct SquareAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for SquareAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = item {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum SquarePaymentStatus {
    Approved,
    Completed,
    Cancelled,
    #[default]
    Failed,
}

impl From<SquarePaymentStatus> for enums::AttemptStatus {
    fn from(item: SquarePaymentStatus) -> Self {
        match item {
            SquarePaymentStatus::Completed => Self::Charged,
            SquarePaymentStatus::Failed => Self::Failure,
            SquarePaymentStatus::Approved => Self::Authorizing,
            SquarePaymentStatus::Cancelled => Self::Failure
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentResponseType {
    status: SquarePaymentStatus,
    id: String,
    updated_at: String,
    amount_money: AmountMoneyType,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SquarePaymentsResponse {
    payment: PaymentResponseType
}

impl<F,T> TryFrom<types::ResponseRouterData<F, SquarePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, SquarePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.payment.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.payment.id),
                redirection_data: None,
                redirect: false,
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
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct SquareRefundRequest {
    payment_id: String,
    idempotency_key: String,
    amount_money: AmountMoneyType,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for SquareRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        let payment_request = Self {
            amount_money: AmountMoneyType {
                amount: item.request.amount,
                currency: item.request.currency.to_string()
            },
            idempotency_key: Uuid::new_v4().to_string(),
            payment_id: item.request.connector_transaction_id
            }
        };
        Ok(payment_request)
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
}

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

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
         todo!()
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SquareErrorResponse {}
