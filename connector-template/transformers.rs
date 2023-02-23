use serde::{Deserialize, Serialize};
use crate::{core::errors,types::{self,api, storage::enums}};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct {{project-name | downcase | pascal_case}}PaymentsRequest {}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for {{project-name | downcase | pascal_case}}PaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        Err(errors::ConnectorError::NotImplemented("try_from PaymentsAuthorizeRouterData".to_string()).into())
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct {{project-name | downcase | pascal_case}}AuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for {{project-name | downcase | pascal_case}}AuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented("try_from ConnectorAuthType".to_string()).into())
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
            {{project-name | downcase | pascal_case}}PaymentStatus::Succeeded => Self::Charged,
            {{project-name | downcase | pascal_case}}PaymentStatus::Failed => Self::Failure,
            {{project-name | downcase | pascal_case}}PaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct {{project-name | downcase | pascal_case}}PaymentsResponse {
    status: {{project-name | downcase | pascal_case}}PaymentStatus,
    id: String,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, {{project-name | downcase | pascal_case}}PaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, {{project-name | downcase | pascal_case}}PaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
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

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct {{project-name | downcase | pascal_case}}RefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for {{project-name | downcase | pascal_case}}RefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Err(errors::ConnectorError::NotImplemented("try_from RefundsRouterData".to_string()).into())
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
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented("try_from RefundsResponseRouterData".to_string()).into())
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Err(errors::ConnectorError::NotImplemented("try_from RefundsResponseRouterData".to_string()).into())
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct {{project-name | downcase | pascal_case}}ErrorResponse {}
