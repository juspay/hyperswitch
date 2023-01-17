use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    core::errors,
    pii::{self, Secret},
    services,
    types::{
        self, api,
        storage::enums,
        transformers::{self, ForeignFrom},
    },
};

#[derive(Default, Debug, Serialize)]
pub struct RapydPaymentsRequest {
    pub amount: i64,
    pub currency: enums::Currency,
    pub payment_method: PaymentMethod,
    pub payment_method_options: PaymentMethodOptions,
    pub capture: bool,
}

#[derive(Default, Debug, Serialize)]
pub struct PaymentMethodOptions {
    #[serde(rename = "3d_required")]
    pub three_ds: bool,
}
#[derive(Default, Debug, Serialize)]
pub struct PaymentMethod {
    #[serde(rename = "type")]
    pub pm_type: String,
    pub fields: PaymentFields,
}

#[derive(Default, Debug, Serialize)]
pub struct PaymentFields {
    pub number: Secret<String, pii::CardNumber>,
    pub expiration_month: Secret<String>,
    pub expiration_year: Secret<String>,
    pub name: Secret<String>,
    pub cvv: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for RapydPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api_models::payments::PaymentMethod::Card(ref ccard) => {
                let payment_method = PaymentMethod {
                    pm_type: "in_amex_card".to_owned(), //[#369] Map payment method type based on country
                    fields: PaymentFields {
                        number: ccard.card_number.to_owned(),
                        expiration_month: ccard.card_exp_month.to_owned(),
                        expiration_year: ccard.card_exp_year.to_owned(),
                        name: ccard.card_holder_name.to_owned(),
                        cvv: ccard.card_cvc.to_owned(),
                    },
                };
                let three_ds_enabled = matches!(item.auth_type, enums::AuthenticationType::ThreeDs);
                let payment_method_options = PaymentMethodOptions {
                    three_ds: three_ds_enabled,
                };
                Ok(Self {
                    amount: item.request.amount,
                    currency: item.request.currency,
                    payment_method,
                    capture: matches!(
                        item.request.capture_method,
                        Some(enums::CaptureMethod::Automatic) | None
                    ),
                    payment_method_options,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum RapydPaymentStatus {
    #[serde(rename = "ACT")]
    Active,
    #[serde(rename = "CAN")]
    CanceledByClientOrBank,
    #[serde(rename = "CLO")]
    Closed,
    #[serde(rename = "ERR")]
    Error,
    #[serde(rename = "EXP")]
    Expired,
    #[serde(rename = "REV")]
    ReversedByRapyd,
    #[default]
    #[serde(rename = "NEW")]
    New,
}

impl From<transformers::Foreign<(RapydPaymentStatus, String)>>
    for transformers::Foreign<enums::AttemptStatus>
{
    fn from(item: transformers::Foreign<(RapydPaymentStatus, String)>) -> Self {
        let (status, next_action) = item.0;
        match status {
            RapydPaymentStatus::Closed => enums::AttemptStatus::Charged,
            RapydPaymentStatus::Active => {
                if next_action == "3d_verification" {
                    enums::AttemptStatus::AuthenticationPending
                } else if next_action == "pending_capture" {
                    enums::AttemptStatus::Authorized
                } else {
                    enums::AttemptStatus::Pending
                }
            }
            RapydPaymentStatus::CanceledByClientOrBank
            | RapydPaymentStatus::Error
            | RapydPaymentStatus::Expired
            | RapydPaymentStatus::ReversedByRapyd => enums::AttemptStatus::Failure,
            RapydPaymentStatus::New => enums::AttemptStatus::Authorizing,
        }
        .into()
    }
}

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
    pub redirect_url: Option<String>,
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
                Some(data) => {
                    let redirection_data = match (data.next_action.as_str(), data.redirect_url) {
                        ("3d_verification", Some(url)) => {
                            let url = Url::parse(&url)
                                .into_report()
                                .change_context(errors::ParsingError)?;
                            let mut base_url = url.clone();
                            base_url.set_query(None);
                            Some(services::RedirectForm {
                                url: base_url.to_string(),
                                method: services::Method::Get,
                                form_fields: std::collections::HashMap::from_iter(
                                    url.query_pairs()
                                        .map(|(k, v)| (k.to_string(), v.to_string())),
                                ),
                            })
                        }
                        (_, _) => None,
                    };
                    (
                        enums::AttemptStatus::foreign_from((data.status, data.next_action)),
                        Ok(types::PaymentsResponseData::TransactionResponse {
                            resource_id: types::ResponseId::ConnectorTransactionId(data.id), //transaction_id is also the field but this id is used to initiate a refund
                            redirect: redirection_data.is_some(),
                            redirection_data,
                            mandate_reference: None,
                            connector_metadata: None,
                        }),
                    )
                }
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
