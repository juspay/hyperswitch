use serde::{Deserialize, Serialize};
use crate::{core::errors, types::{self, api, storage::enums}};
use masking::PeekInterface;

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FortePaymentsRequest {
    pub authorization_amount: f64,
    pub subtotal_amount: f64,
    pub billing_address: BillingAddress,
    pub card: ForteCardRequest
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BillingAddress {
    pub first_name: String,
    pub last_name: String
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForteCardRequest {
    pub card_type: String,
    pub name_on_card: String,
    pub account_number: String,
    pub expire_month: String,
    pub expire_year: String,
    pub card_verification_value: String
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethodData::Card(ref req_card) => {
                let request = FortePaymentsRequest{
                    billing_address : BillingAddress { first_name: req_card.card_holder_name.peek().clone(), last_name: req_card.card_holder_name.peek().clone() },
                    card: ForteCardRequest {
                        card_type: String::from("visa"),
                        name_on_card: req_card.card_holder_name.peek().clone(),
                        account_number: req_card.card_number.peek().clone(),
                        expire_month: req_card.card_exp_month.peek().clone(),
                        expire_year: req_card.card_exp_year.peek().clone().to_string(),
                        card_verification_value: req_card.card_cvc.peek().clone().to_string(),
                    },
                    authorization_amount: item.request.amount as f64,
                    subtotal_amount: item.request.amount as f64,
                };
                Ok(request)
            }
            _ => Err(
                errors::ConnectorError::NotImplemented("Payment Method".to_string()).into(),
            ),
    }
}
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct ForteAuthType {
     pub api_key: String,
     pub api_secret: String
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_string(),
                api_secret: key1.to_string()
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
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

//Res Types Start
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FortePaymentsResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub action: String,
    pub authorization_amount: f64,
    pub authorization_code: String,
    pub entered_by: String,
    pub billing_address: BillingAddress,
    pub card: ForteCardResponse,
    pub response: ForteResponseStruct,
    pub links: ForteLinks
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForteCardResponse {
    pub name_on_card: String,
    pub last_4_account_number: String,
    pub masked_account_number: String,
    pub expire_month: i32,
    pub expire_year: i32,
    pub card_type: String
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForteResponseStruct {
    pub environment: String,
    pub response_type: String,
    pub response_code: String,
    pub response_desc: String,
    pub authorization_code: String,
    pub avs_result: String,
    pub cvv_result: String
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForteLinks {
    pub disputes: String,
    pub settlements: String,
     #[serde(rename = "self")]
    pub _self: String
}

impl<F,T> TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        let status_string = String::from(item.response.response.response_desc);
        Ok(Self {
            status: if status_string == "APPROVAL" {  enums::AttemptStatus::Authorized} else { enums::AttemptStatus::Pending },
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.transaction_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForteCapturePaymentRequest {
    pub action: String,
    pub transaction_id: String,
    pub authorization_code: String
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForteCapturePaymentResponse {
    pub transaction_id: String,
    pub location_id: String,
    pub original_transaction_id: String,
    pub action: String,
    pub authorization_code: String,
    pub entered_by: String,
    pub response: CaptureResponseStruct
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CaptureResponseStruct {
    pub environment: String,
    pub response_type: String,
    pub response_code: String,
    pub response_desc: String,
    pub authorization_code: String
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
pub struct ForteErrorResponse {}