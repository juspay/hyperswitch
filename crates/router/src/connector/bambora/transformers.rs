use serde::{Deserialize, Serialize};
use crate::{core::errors,types::{self,api, storage::enums}};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BamboraPaymentsRequest {
    amount: i32,
    payment_method: String,
    card: Card
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct Card {
    name: String,
    number: String,
    expiry_month: String,
    expiry_year: String,
    cvd: String
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BamboraPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        todo!()
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct BamboraAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for BamboraAuthType  {
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
    #[serde(rename = "0")]
    ZERO,
    #[serde(rename = "1")]
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BamboraPaymentsResponse {
    approved: BamboraPaymentStatus,
    id: String,
    response: String
}

impl<F,T> TryFrom<types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
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

// //TODO: Fill the struct with respective fields
// // REFUND :
// // Type definition for RefundRequest
// #[derive(Default, Debug, Serialize)]
// pub struct BamboraRefundRequest {}

// impl<F> TryFrom<&types::RefundsRouterData<F>> for BamboraRefundRequest {
//     type Error = error_stack::Report<errors::ParsingError>;
//     fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
//        todo!()
//     }
// }

// // Type definition for Refund Response

// #[allow(dead_code)]
// #[derive(Debug, Serialize, Default, Deserialize, Clone)]
// pub enum RefundStatus {
//     Succeeded,
//     Failed,
//     #[default]
//     Processing,
// }

// impl From<RefundStatus> for enums::RefundStatus {
//     fn from(item: RefundStatus) -> Self {
//         match item {
//             RefundStatus::Succeeded => Self::Success,
//             RefundStatus::Failed => Self::Failure,
//             RefundStatus::Processing => Self::Pending,
//             //TODO: Review mapping
//         }
//     }
// }

// //TODO: Fill the struct with respective fields
// #[derive(Default, Debug, Clone, Serialize, Deserialize)]
// pub struct RefundResponse {
// }

// impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
//     for types::RefundsRouterData<api::Execute>
// {
//     type Error = error_stack::Report<errors::ParsingError>;
//     fn try_from(
//         _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
//     ) -> Result<Self, Self::Error> {
//         todo!()
//     }
// }

// impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
// {
//      type Error = error_stack::Report<errors::ParsingError>;
//     fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
//          todo!()
//      }
//  }

// //TODO: Fill the struct with respective fields
// #[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
// pub struct BamboraErrorResponse {}
