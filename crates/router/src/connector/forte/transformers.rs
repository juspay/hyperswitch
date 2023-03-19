use error_stack::IntoReport;
use serde::{Deserialize, Serialize};
use masking::Secret;
use crate::{connector::utils::PaymentsAuthorizeRequestData,core::errors,types::{self,api, storage::enums}};
use crate::types::api::enums::CardNetwork;

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct FortePaymentsRequest {
    pub action: String,
    pub authorization_amount: String,
    pub billing_address: BillingAddress,
    pub card: ForteCard
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BillingAddress {
    pub first_name: String,
    pub last_name: String
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ForteCard {
    pub card_type: Option<CardNetwork>,
    pub name_on_card: Secret<String>,
    pub account_number: Secret<String, common_utils::pii::CardNumber>,
    pub expire_month: Secret<String>,
    pub expire_year: Secret<String>,
    pub card_verification_value: Secret<String>
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card = ForteCard {
                    card_type: req_card.card_network,
                    name_on_card: req_card.card_holder_name,
                    account_number: req_card.card_number,
                    expire_month: req_card.card_exp_month,
                    expire_year: req_card.card_exp_year,
                    card_verification_value: req_card.card_cvc,
                };
                Ok(Self {
                    action: "sale".to_string(),
                    billing_address:BillingAddress { first_name: "Kritik".to_string().into(), last_name: "Modi".to_string().into() },
                    authorization_amount: item.request.amount.to_string(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }    
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct ForteAuthType {
    pub(super) api_key: String,
    pub(super) key1: String
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_string(),
                key1: key1.to_string()
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FortePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<FortePaymentStatus> for enums::AttemptStatus {
    fn from(item: FortePaymentStatus) -> Self {
        match item {
            FortePaymentStatus::Succeeded => Self::Charged,
            FortePaymentStatus::Failed => Self::Failure,
            FortePaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FortePaymentsResponse {
    status: FortePaymentStatus,
    id: String,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
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
pub struct ForteRefundRequest {
    pub amount: i64
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ForteRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            amount: item.request.amount,
        })
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
    id: String,
    status: RefundStatus
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ForteErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
