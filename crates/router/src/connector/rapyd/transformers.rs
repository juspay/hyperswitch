use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    consts,
    core::errors,
    pii::{self, Secret},
    services,
    types::{
        self, api,
        storage::enums,
        transformers::{self, ForeignFrom},
    },
    utils::OptionExt,
};

#[derive(Default, Debug, Serialize)]
pub struct RapydPaymentsRequest {
    pub amount: i64,
    pub currency: enums::Currency,
    pub payment_method: PaymentMethod,
    pub payment_method_options: Option<PaymentMethodOptions>,
    pub capture: Option<bool>,
    pub description: Option<String>,
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
    pub fields: Option<PaymentFields>,
    pub address: Option<Address>,
    pub digital_wallet: Option<RapydWallet>,
}

#[derive(Default, Debug, Serialize)]
pub struct PaymentFields {
    pub number: Secret<String, pii::CardNumber>,
    pub expiration_month: Secret<String>,
    pub expiration_year: Secret<String>,
    pub name: Secret<String>,
    pub cvv: Secret<String>,
}

#[derive(Default, Debug, Serialize)]
pub struct Address {
    name: String,
    line_1: String,
    line_2: Option<String>,
    line_3: Option<String>,
    city: Option<String>,
    state: Option<String>,
    country: Option<String>,
    zip: Option<String>,
    phone_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RapydWallet {
    #[serde(rename = "type")]
    payment_type: String,
    #[serde(rename = "details")]
    apple_pay_token: Option<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for RapydPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let (capture, payment_method_options) = match item.payment_method {
            storage_models::enums::PaymentMethodType::Card => {
                let three_ds_enabled = matches!(item.auth_type, enums::AuthenticationType::ThreeDs);
                let payment_method_options = PaymentMethodOptions {
                    three_ds: three_ds_enabled,
                };
                (
                    Some(matches!(
                        item.request.capture_method,
                        Some(enums::CaptureMethod::Automatic) | None
                    )),
                    Some(payment_method_options),
                )
            }
            _ => (None, None),
        };
        let payment_method = match item.request.payment_method_data {
            api_models::payments::PaymentMethod::Card(ref ccard) => {
                Some(PaymentMethod {
                    pm_type: "in_amex_card".to_owned(), //[#369] Map payment method type based on country
                    fields: Some(PaymentFields {
                        number: ccard.card_number.to_owned(),
                        expiration_month: ccard.card_exp_month.to_owned(),
                        expiration_year: ccard.card_exp_year.to_owned(),
                        name: ccard.card_holder_name.to_owned(),
                        cvv: ccard.card_cvc.to_owned(),
                    }),
                    address: None,
                    digital_wallet: None,
                })
            }
            api_models::payments::PaymentMethod::Wallet(ref wallet_data) => {
                let digital_wallet = match wallet_data.issuer_name {
                    api_models::enums::WalletIssuer::GooglePay => Some(RapydWallet {
                        payment_type: "google_pay".to_string(),
                        apple_pay_token: wallet_data.token.to_owned(),
                    }),
                    api_models::enums::WalletIssuer::ApplePay => Some(RapydWallet {
                        payment_type: "apple_pay".to_string(),
                        apple_pay_token: wallet_data.token.to_owned(),
                    }),
                    _ => None,
                };
                Some(PaymentMethod {
                    pm_type: "by_visa_card".to_string(), //[#369]
                    fields: None,
                    address: None,
                    digital_wallet,
                })
            }
            _ => None,
        }
        .get_required_value("payment_method not implemnted")
        .change_context(errors::ConnectorError::NotImplemented(
            "payment_method".to_owned(),
        ))?;
        Ok(Self {
            amount: item.request.amount,
            currency: item.request.currency,
            payment_method,
            capture,
            payment_method_options,
            description: None,
        })
    }
}

#[derive(Debug, Deserialize)]
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

impl From<transformers::Foreign<(RapydPaymentStatus, NextAction)>>
    for transformers::Foreign<enums::AttemptStatus>
{
    fn from(item: transformers::Foreign<(RapydPaymentStatus, NextAction)>) -> Self {
        let (status, next_action) = item.0;
        match (status, next_action) {
            (RapydPaymentStatus::Closed, _) => enums::AttemptStatus::Charged,
            (RapydPaymentStatus::Active, NextAction::ThreedsVerification) => {
                enums::AttemptStatus::AuthenticationPending
            }
            (RapydPaymentStatus::Active, NextAction::PendingCapture) => {
                enums::AttemptStatus::Authorized
            }
            (
                RapydPaymentStatus::CanceledByClientOrBank
                | RapydPaymentStatus::Expired
                | RapydPaymentStatus::ReversedByRapyd,
                _,
            ) => enums::AttemptStatus::Voided,
            (RapydPaymentStatus::Error, _) => enums::AttemptStatus::Failure,
            (RapydPaymentStatus::New, _) => enums::AttemptStatus::Authorizing,
        }
        .into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RapydPaymentsResponse {
    pub status: Status,
    pub data: Option<ResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub error_code: String,
    pub status: Option<String>,
    pub message: Option<String>,
    pub response_code: Option<String>,
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NextAction {
    #[serde(rename = "3d_verification")]
    ThreedsVerification,
    #[serde(rename = "pending_capture")]
    PendingCapture,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseData {
    pub id: String,
    pub amount: i64,
    pub status: RapydPaymentStatus,
    pub next_action: NextAction,
    pub redirect_url: Option<Url>,
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

#[derive(Default, Debug, Serialize)]
pub struct RapydRefundRequest {
    pub payment: String,
    pub amount: Option<i64>,
    pub currency: Option<enums::Currency>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RapydRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub status: Status,
    pub data: Option<RefundResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    type Error = error_stack::Report<errors::ConnectorError>;
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
    type Error = error_stack::Report<errors::ConnectorError>;
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
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount_to_capture,
            receipt_email: None,
            statement_descriptor: None,
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, RapydPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, RapydPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, response) = match &item.response.data {
            Some(data) => {
                let attempt_status = enums::AttemptStatus::foreign_from((
                    data.status.to_owned(),
                    data.next_action.to_owned(),
                ));
                match attempt_status {
                    storage_models::enums::AttemptStatus::Failure => (
                        enums::AttemptStatus::Failure,
                        Err(types::ErrorResponse {
                            code: data
                                .failure_code
                                .to_owned()
                                .unwrap_or(item.response.status.error_code),
                            status_code: item.http_code,
                            message: item.response.status.status.unwrap_or_default(),
                            reason: data.failure_message.to_owned(),
                        }),
                    ),
                    _ => {
                        let redirection_data = data.redirect_url.as_ref().map(|redirect_url| {
                            services::RedirectForm::from((
                                redirect_url.to_owned(),
                                services::Method::Get,
                            ))
                        });
                        (
                            attempt_status,
                            Ok(types::PaymentsResponseData::TransactionResponse {
                                resource_id: types::ResponseId::ConnectorTransactionId(
                                    data.id.to_owned(),
                                ), //transaction_id is also the field but this id is used to initiate a refund
                                redirection_data,
                                mandate_reference: None,
                                connector_metadata: None,
                            }),
                        )
                    }
                }
            }
            None => (
                enums::AttemptStatus::Failure,
                Err(types::ErrorResponse {
                    code: item.response.status.error_code,
                    status_code: item.http_code,
                    message: item.response.status.status.unwrap_or_default(),
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

#[derive(Debug, Deserialize)]
pub struct RapydIncomingWebhook {
    pub id: String,
    #[serde(rename = "type")]
    pub webhook_type: RapydWebhookObjectEventType,
    pub data: WebhookData,
    pub trigger_operation_id: Option<String>,
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RapydWebhookObjectEventType {
    PaymentCompleted,
    PaymentCaptured,
    PaymentFailed,
    RefundCompleted,
    PaymentRefundRejected,
    PaymentRefundFailed,
}

impl TryFrom<RapydWebhookObjectEventType> for api::IncomingWebhookEvent {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: RapydWebhookObjectEventType) -> Result<Self, Self::Error> {
        match value {
            RapydWebhookObjectEventType::PaymentCompleted => Ok(Self::PaymentIntentSuccess),
            RapydWebhookObjectEventType::PaymentCaptured => Ok(Self::PaymentIntentSuccess),
            RapydWebhookObjectEventType::PaymentFailed => Ok(Self::PaymentIntentFailure),
            _ => Err(errors::ConnectorError::WebhookEventTypeNotFound).into_report()?,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum WebhookData {
    PaymentData(ResponseData),
    RefundData(RefundResponseData),
}

impl From<ResponseData> for RapydPaymentsResponse {
    fn from(value: ResponseData) -> Self {
        Self {
            status: Status {
                error_code: consts::NO_ERROR_CODE.to_owned(),
                status: None,
                message: None,
                response_code: None,
                operation_id: None,
            },
            data: Some(value),
        }
    }
}

impl From<RefundResponseData> for RefundResponse {
    fn from(value: RefundResponseData) -> Self {
        Self {
            status: Status {
                error_code: consts::NO_ERROR_CODE.to_owned(),
                status: None,
                message: None,
                response_code: None,
                operation_id: None,
            },
            data: Some(value),
        }
    }
}
