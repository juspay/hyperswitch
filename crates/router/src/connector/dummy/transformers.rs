use masking::PeekInterface;
use serde::{Deserialize, Serialize};

use crate::{
    consts,
    core::errors,
    services,
    types::{self, api, storage::enums},
};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DummyPaymentsRequest {}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for DummyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented(
            "try_from PaymentsAuthorizeRouterData".to_string(),
        )
        .into())
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct DummyAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for DummyAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented("try_from ConnectorAuthType".to_string()).into())
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DummyPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
    RequiresCustomerAction,
}

impl From<DummyPaymentStatus> for enums::AttemptStatus {
    fn from(item: DummyPaymentStatus) -> Self {
        match item {
            DummyPaymentStatus::Succeeded => Self::Charged,
            DummyPaymentStatus::Failed => Self::Failure,
            DummyPaymentStatus::Processing => Self::Authorizing,
            DummyPaymentStatus::RequiresCustomerAction => Self::AuthenticationPending,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DummyPaymentsResponse {
    pub status: DummyPaymentStatus,
    pub transaction_id: String,
    pub redirection_data: Option<services::RedirectForm>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub status_code: u16,
}

impl TryFrom<&types::PaymentsAuthorizeData> for DummyPaymentsResponse {
    type Error = errors::ConnectorError;

    fn try_from(item: &types::PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        let transaction_id = common_utils::generate_id_with_default_len("test");
        match item.payment_method_data {
            api_models::payments::PaymentMethodData::Card(ref card_data) => {
                let raw_card_number = card_data.card_number.peek();
                match raw_card_number.as_str() {
                    "4242424242424242" => Ok(Self {
                        status: DummyPaymentStatus::Succeeded,
                        transaction_id,
                        redirection_data: None,
                        error_code: None,
                        error_message: None,
                        status_code: 200,
                    }),
                    _ => Ok(Self {
                        status: DummyPaymentStatus::Failed,
                        transaction_id,
                        redirection_data: None,
                        error_code: Some("Invalid Card".to_string()),
                        error_message: Some(
                            "Using actual card numbers in test enviromnent is not possible"
                                .to_string(),
                        ),
                        status_code: 400,
                    }),
                }
            }
            api_models::payments::PaymentMethodData::Wallet(_) => todo!(),
            api_models::payments::PaymentMethodData::PayLater(_) => todo!(),
            api_models::payments::PaymentMethodData::BankRedirect(_) => todo!(),
        }
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, DummyPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, DummyPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = match item.response.status {
            DummyPaymentStatus::Failed => Err(types::ErrorResponse {
                code: item
                    .response
                    .error_code
                    .unwrap_or(consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .error_message
                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                reason: None,
                status_code: item.response.status_code,
            }),
            _ => Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
        };
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response,
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct DummyRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for DummyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
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
pub struct RefundResponse {}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented(
            "try_from RefundsResponseRouterData".to_string(),
        )
        .into())
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Err(errors::ConnectorError::NotImplemented(
            "try_from RefundsResponseRouterData".to_string(),
        )
        .into())
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DummyErrorResponse {}
