use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    pii::PeekInterface,
    types::{
        self, api,
        storage::enums,
        transformers::{self, ForeignFrom},
    },
};

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
                    pm_type: "in_amex_card".to_owned(), //TODO Mapping of our pm_type to rapyd
                    fields: PaymentFields {
                        number: ccard.card_number.peek().to_string(),
                        expiration_month: ccard.card_exp_month.peek().to_string(),
                        expiration_year: ccard.card_exp_year.peek().to_string(),
                        name: ccard.card_holder_name.peek().to_string(),
                        cvv: ccard.card_cvc.peek().to_string(),
                    },
                };
                Ok(Self {
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
#[allow(clippy::upper_case_acronyms)]
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

impl From<transformers::Foreign<(RapydPaymentStatus, String)>>
    for transformers::Foreign<enums::AttemptStatus>
{
    fn from(item: transformers::Foreign<(RapydPaymentStatus, String)>) -> Self {
        let (status, next_action) = item.0;
        match status {
            RapydPaymentStatus::CLO => enums::AttemptStatus::Charged,
            RapydPaymentStatus::ACT => {
                if next_action == "3d_verification" {
                    enums::AttemptStatus::AuthenticationPending
                } else if next_action == "pending_capture" {
                    enums::AttemptStatus::Authorized
                } else {
                    enums::AttemptStatus::Pending
                }
            }
            RapydPaymentStatus::CAN
            | RapydPaymentStatus::ERR
            | RapydPaymentStatus::EXP
            | RapydPaymentStatus::REV => enums::AttemptStatus::Failure,
            RapydPaymentStatus::NEW => enums::AttemptStatus::Authorizing,
        }
        .into()
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
    pub next_action: String,
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
                    enums::AttemptStatus::foreign_from((data.status, data.next_action)),
                    Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(data.id), //transaction_id is also the field but this id is used to initiate a refund
                        redirection_data: None,
                        redirect: false,
                        mandate_reference: None,
                        connector_metadata: None,
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
pub struct RapydRefundRequest {
    pub payment: String,
    pub amount: Option<i64>,
    pub currency: Option<enums::Currency>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RapydRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            payment: item.request.connector_transaction_id.to_string(),
            amount: Some(item.request.amount),
            currency: Some(item.request.currency),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum RefundStatus {
    Completed,
    Error,
    Rejected,
    #[default]
    Pending,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Completed => Self::Success,
            RefundStatus::Error | RefundStatus::Rejected => Self::Failure,
            RefundStatus::Pending => Self::Pending,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub status: Status,
    pub data: Option<RefundResponseData>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefundResponseData {
    //Some field related to forign exchange and split payment can be added as and when implemented
    pub id: String,
    pub payment: String,
    pub amount: i64,
    pub currency: enums::Currency,
    pub status: RefundStatus,
    pub created_at: Option<i64>,
    pub failure_reason: Option<String>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let (connector_refund_id, refund_status) = match item.response.data {
            Some(data) => (data.id, enums::RefundStatus::from(data.status)),
            None => (
                item.response.status.error_code,
                enums::RefundStatus::Failure,
            ),
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let (connector_refund_id, refund_status) = match item.response.data {
            Some(data) => (data.id, enums::RefundStatus::from(data.status)),
            None => (
                item.response.status.error_code,
                enums::RefundStatus::Failure,
            ),
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct CaptureRequest {
    amount: Option<i64>,
    receipt_email: Option<String>,
    statement_descriptor: Option<String>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for CaptureRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount_to_capture,
            receipt_email: None,
            statement_descriptor: None,
        })
    }
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<RapydPaymentsResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<RapydPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let (status, response) = match item.response.status.status.as_str() {
            "SUCCESS" => match item.response.data {
                Some(data) => (
                    enums::AttemptStatus::foreign_from((data.status, data.next_action)),
                    Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(data.id), //transaction_id is also the field but this id is used to initiate a refund
                        redirection_data: None,
                        redirect: false,
                        mandate_reference: None,
                        connector_metadata: None,
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

impl TryFrom<types::PaymentsCancelResponseRouterData<RapydPaymentsResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::PaymentsCancelResponseRouterData<RapydPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let (status, response) = match item.response.status.status.as_str() {
            "SUCCESS" => match item.response.data {
                Some(data) => (
                    enums::AttemptStatus::foreign_from((data.status, data.next_action)),
                    Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(data.id), //transaction_id is also the field but this id is used to initiate a refund
                        redirection_data: None,
                        redirect: false,
                        mandate_reference: None,
                        connector_metadata: None,
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
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct RapydErrorResponse {}
