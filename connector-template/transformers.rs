use serde::{Deserialize, Serialize};
use crate::{core::errors,pii::PeekInterface,types::{self,api, storage::enums}};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct {{project-name | downcase | pascal_case}}PaymentsRequest {}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for {{project-name | downcase | pascal_case}}PaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        todo!()
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct {{project-name | downcase | pascal_case}}AuthType {}

impl TryFrom<&types::ConnectorAuthType> for {{project-name | downcase | pascal_case}}AuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        todo!()
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum {{project-name | downcase | pascal_case}}PaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<{{project-name | downcase | pascal_case}}PaymentStatus> for enums::AttemptStatus {
    fn from(item: {{project-name | downcase | pascal_case}}PaymentStatus) -> Self {
        match item {
            {{project-name | downcase | pascal_case}}PaymentStatus::Succeeded => enums::AttemptStatus::Charged,
            {{project-name | downcase | pascal_case}}PaymentStatus::Failed => enums::AttemptStatus::Failure,
            {{project-name | downcase | pascal_case}}PaymentStatus::Processing => enums::AttemptStatus::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct {{project-name | downcase | pascal_case}}PaymentsResponse {}

impl TryFrom<types::PaymentsResponseRouterData<{{project-name | downcase | pascal_case}}PaymentsResponse>> for types::PaymentsAuthorizeRouterData {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::PaymentsResponseRouterData<{{project-name | downcase | pascal_case}}PaymentsResponse>) -> Result<Self,Self::Error> {
        todo!()
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct {{project-name | downcase | pascal_case}}RefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for {{project-name | downcase | pascal_case}}RefundRequest {
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

impl From<self::RefundStatus> for enums::RefundStatus {
    fn from(item: self::RefundStatus) -> Self {
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
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
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
pub struct {{project-name | downcase | pascal_case}}ErrorResponse {}
