use crate::{
    core::errors,
    pii::PeekInterface,
    types::{self, api, storage::enums},
};
use serde::{Deserialize, Serialize};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize)]
pub struct RapydPaymentsRequest {
    pub amount: i64,
    pub currency: enums::Currency,
    pub payment_method: PaymentMethod,
    pub capture: bool,
}

#[derive(Default, Debug, Serialize)]
pub struct PaymentMethod {
    #[serde(rename = "type")]
    pub pm_type: String,
    pub fields: PaymentFields,
}

#[derive(Default, Debug, Serialize)]
pub struct PaymentFields {
    pub number: String,
    pub expiration_month: String,
    pub expiration_year: String,
    pub name: String,
    pub cvv: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for RapydPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api_models::payments::PaymentMethod::Card(ref ccard) => {
                let payment_method = PaymentMethod {
                    pm_type: "in_amex_card".to_owned(), //TODO
                    fields: PaymentFields {
                        number: ccard.card_number.peek().to_string(),
                        expiration_month: ccard.card_exp_month.peek().to_string(),
                        expiration_year: ccard.card_exp_year.peek().to_string(),
                        name: ccard.card_holder_name.peek().to_string(),
                        cvv: ccard.card_cvc.peek().to_string(),
                    },
                };
                Ok(RapydPaymentsRequest {
                    amount: item.request.amount,
                    currency: item.request.currency,
                    payment_method,
                    capture: matches!(
                        item.request.capture_method,
                        Some(enums::CaptureMethod::Automatic) | None
                    ),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct RapydAuthType {
    pub access_key: String,
    pub secret_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for RapydAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                access_key: api_key.to_string(),
                secret_key: key1.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RapydPaymentStatus {
    ACT,
    CAN,
    CLO,
    ERR,
    EXP,
    REV,
    #[default]
    NEW,
}

impl From<RapydPaymentStatus> for enums::AttemptStatus {
    fn from(item: RapydPaymentStatus) -> Self {
        match item {
            RapydPaymentStatus::CLO => enums::AttemptStatus::Charged,
            RapydPaymentStatus::ACT => enums::AttemptStatus::AuthenticationPending,
            RapydPaymentStatus::CAN
            | RapydPaymentStatus::ERR
            | RapydPaymentStatus::EXP
            | RapydPaymentStatus::REV => enums::AttemptStatus::Failure,
            RapydPaymentStatus::NEW => enums::AttemptStatus::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RapydPaymentsResponse {
    pub status: Status,
    pub data: Option<ResponseData>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub error_code: String,
    pub status: String,
    pub message: Option<String>,
    pub response_code: Option<String>,
    pub operation_id: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseData {
    pub id: String,
    pub amount: i64,
    pub status: RapydPaymentStatus,
    pub original_amount: Option<i64>,
    pub is_partial: Option<bool>,
    pub currency_code: Option<enums::Currency>,
    pub country_code: Option<String>,
    pub captured: Option<bool>,
    pub transaction_id: String,
    pub paid: Option<bool>,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
}

impl TryFrom<types::PaymentsResponseRouterData<RapydPaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<RapydPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let (status, response) = match item.response.status.status.as_str() {
            "SUCCESS" => match item.response.data {
                Some(data) => (
                    data.status.into(),
                    Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(data.transaction_id),
                        redirection_data: None,
                        redirect: false,
                        mandate_reference: None,
                    }),
                ),
                None => (
                    enums::AttemptStatus::Failure,
                    Err(types::ErrorResponse {
                        code: item.response.status.error_code,
                        message: item.response.status.status,
                        reason: item.response.status.message,
                    }),
                ),
            },
            "ERROR" => (
                enums::AttemptStatus::Failure,
                Err(types::ErrorResponse {
                    code: item.response.status.error_code,
                    message: item.response.status.status,
                    reason: item.response.status.message,
                }),
            ),
            _ => (
                enums::AttemptStatus::Failure,
                Err(types::ErrorResponse {
                    code: item.response.status.error_code,
                    message: item.response.status.status,
                    reason: item.response.status.message,
                }),
            ),
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct RapydRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RapydRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct RapydErrorResponse {}
