use serde::{Deserialize, Serialize};
use crate::{core::errors,types::{self,storage::enums}};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct {{project-name | downcase | pascal_case}}PaymentsRequest {}

impl TryFrom<&types::PaymentsRouterData> for {{project-name | downcase | pascal_case}}PaymentsRequest  {
    type Error = error_stack::Report<errors::ValidateError>;
    fn try_from(_item: &types::PaymentsRouterData) -> Result<Self,Self::Error> {
        todo!()
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct {{project-name | downcase | pascal_case}}AuthType {}

impl TryFrom<&types::ConnectorAuthType> for {{project-name | downcase | pascal_case}}AuthType  {
    type Error = error_stack::Report<errors::ValidateError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        todo!()
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum {{project-name | downcase | pascal_case}}PaymentStatus {
    Succeeded,
    Failed,
    Processing,
}

// Default should be Processing
impl Default for {{project-name | downcase | pascal_case}}PaymentStatus {
    fn default() -> Self {
        {{project-name | downcase | pascal_case}}PaymentStatus::Processing
    }
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

impl TryFrom<types::PaymentsResponseRouterData<{{project-name | downcase | pascal_case}}PaymentsResponse>> for types::PaymentsRouterData {
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

impl TryFrom<&types::RefundsRouterData> for {{project-name | downcase | pascal_case}}RefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData) -> Result<Self,Self::Error> {
       todo!()
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    Processing,
}

// Default should be Processing
impl Default for RefundStatus {
    fn default() -> Self {
        RefundStatus::Processing
    }
}

impl From<self::RefundStatus> for enums::RefundStatus {
    fn from(item: self::RefundStatus) -> Self {
        match item {
            self::RefundStatus::Succeeded => enums::RefundStatus::Success,
            self::RefundStatus::Failed => enums::RefundStatus::Failure,
            self::RefundStatus::Processing => enums::RefundStatus::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct {{project-name | downcase | pascal_case}}RefundResponse {}

impl TryFrom<types::RefundsResponseRouterData<{{project-name | downcase | pascal_case}}RefundResponse>> for types::RefundsRouterData {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::RefundsResponseRouterData<{{project-name | downcase | pascal_case}}RefundResponse>) -> Result<Self,Self::Error> {
        todo!()
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct {{project-name | downcase | pascal_case}}ErrorResponse {}
