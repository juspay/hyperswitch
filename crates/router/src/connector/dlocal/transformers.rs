use serde::{Deserialize, Serialize};
use crate::{core::errors,types::{self,api, storage::enums}};
use self::{storage::enums as storage_enums};

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct Payer {
    pub name : String,
    pub email: String,
    pub document: String,
}

#[derive(Debug, Default, Eq, PartialEq, Serialize)]
pub struct Card {
    pub holder_name: String,
    pub number: String,
    pub cvv: String,
    pub expiration_month: i32,
    pub expiration_year: i32,
    pub capture: String,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DlocalPaymentsRequest {
    pub amount: i64, //amount in cents, hence passed as integer
    pub currency: storage_enums::Currency,
    pub country: Option<String>,
    pub payment_method_id: String,
    pub payment_method_flow: String,
    pub payer: Payer,
    pub card: Card,
    pub order_id: String,
    pub notification_url: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for DlocalPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let amount = item.request.amount,
        let currency = item.request.currency,

    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct DlocalAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for DlocalAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        todo!()
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DlocalPaymentStatus {
    AUTHORIZED,
    PAID,
    VERIFIED,
    CANCELLED,
    #[default]
    PENDING,
}

impl From<DlocalPaymentStatus> for enums::AttemptStatus {
    fn from(item: DlocalPaymentStatus) -> Self {
        match item {
            DlocalPaymentStatus::AUTHORIZED => Self::Authorized,
            DlocalPaymentStatus::VERIFIED => Self::Authorized,
            DlocalPaymentStatus::PAID => Self::Charged,
            DlocalPaymentStatus::PENDING => Self::Pending,
            DlocalPaymentStatus::CANCELLED => Self::Voided
        }
    }
}

//TODO: Fill the struct with respective fields
// {
//     "id": "D-4-e2227981-8ec8-48fd-8e9a-19fedb08d73a",
//     "amount": 120,
//     "currency": "USD",
//     "payment_method_id": "CARD",
//     "payment_method_type": "CARD",
//     "payment_method_flow": "DIRECT",
//     "country": "BR",
//     "card": {
//         "holder_name": "Thiago Gabriel",
//         "expiration_month": 10,
//         "expiration_year": 2040,
//         "brand": "VI",
//         "last4": "1111"
//     },
//     "created_date": "2019-02-06T21:04:43.000+0000",
//     "approved_date": "2019-02-06T21:04:44.000+0000",
//     "status": "AUTHORIZED",
//     "status_detail": "The payment was authorized",
//     "status_code": "600",
//     "order_id": "657434343",
//     "notification_url": "http://merchant.com/notifications"
// }
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlocalPaymentsResponse {
    status: DlocalPaymentStatus,
    id: String,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, DlocalPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, DlocalPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
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

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct DlocalRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for DlocalRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
       todo!()
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
pub struct DlocalErrorResponse {}
