use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;

use crate::{
    connector::utils::PaymentsAuthorizeRequestData,
    consts,
    core::errors,
    pii::Secret,
    services,
    types::{self, api, storage::enums, transformers::ForeignFrom},
    utils::OptionExt,
};

#[derive(Debug, Serialize)]
pub struct RapydRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for RapydRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct RapydPaymentsRequest {
    pub amount: i64,
    pub currency: enums::Currency,
    pub payment_method: PaymentMethod,
    pub payment_method_options: Option<PaymentMethodOptions>,
    pub capture: Option<bool>,
    pub description: Option<String>,
    pub complete_payment_url: Option<String>,
    pub error_payment_url: Option<String>,
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
    pub number: cards::CardNumber,
    pub expiration_month: Secret<String>,
    pub expiration_year: Secret<String>,
    pub name: Secret<String>,
    pub cvv: Secret<String>,
}

#[derive(Default, Debug, Serialize)]
pub struct Address {
    name: Secret<String>,
    line_1: Secret<String>,
    line_2: Option<Secret<String>>,
    line_3: Option<Secret<String>>,
    city: Option<String>,
    state: Option<Secret<String>>,
    country: Option<String>,
    zip: Option<String>,
    phone_number: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RapydWallet {
    #[serde(rename = "type")]
    payment_type: String,
    #[serde(rename = "details")]
    token: Option<String>,
}

impl TryFrom<&RapydRouterData<&types::PaymentsAuthorizeRouterData>> for RapydPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RapydRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let (capture, payment_method_options) = match item.router_data.payment_method {
            diesel_models::enums::PaymentMethod::Card => {
                let three_ds_enabled = matches!(
                    item.router_data.auth_type,
                    enums::AuthenticationType::ThreeDs
                );
                let payment_method_options = PaymentMethodOptions {
                    three_ds: three_ds_enabled,
                };
                (
                    Some(matches!(
                        item.router_data.request.capture_method,
                        Some(enums::CaptureMethod::Automatic) | None
                    )),
                    Some(payment_method_options),
                )
            }
            _ => (None, None),
        };
        let payment_method = match item.router_data.request.payment_method_data {
            api_models::payments::PaymentMethodData::Card(ref ccard) => {
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
            api_models::payments::PaymentMethodData::Wallet(ref wallet_data) => {
                let digital_wallet = match wallet_data {
                    api_models::payments::WalletData::GooglePay(data) => Some(RapydWallet {
                        payment_type: "google_pay".to_string(),
                        token: Some(data.tokenization_data.token.to_owned()),
                    }),
                    api_models::payments::WalletData::ApplePay(data) => Some(RapydWallet {
                        payment_type: "apple_pay".to_string(),
                        token: Some(data.payment_data.to_string()),
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
        let return_url = item.router_data.request.get_return_url()?;
        Ok(Self {
            amount: item.amount,
            currency: item.router_data.request.currency,
            payment_method,
            capture,
            payment_method_options,
            description: None,
            error_payment_url: Some(return_url.clone()),
            complete_payment_url: Some(return_url),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct RapydAuthType {
    pub access_key: Secret<String>,
    pub secret_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for RapydAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                access_key: api_key.to_owned(),
                secret_key: key1.to_owned(),
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

impl ForeignFrom<(RapydPaymentStatus, NextAction)> for enums::AttemptStatus {
    fn foreign_from(item: (RapydPaymentStatus, NextAction)) -> Self {
        let (status, next_action) = item;
        match (status, next_action) {
            (RapydPaymentStatus::Closed, _) => Self::Charged,
            (
                RapydPaymentStatus::Active,
                NextAction::ThreedsVerification | NextAction::PendingConfirmation,
            ) => Self::AuthenticationPending,
            (
                RapydPaymentStatus::Active,
                NextAction::PendingCapture | NextAction::NotApplicable,
            ) => Self::Authorized,
            (
                RapydPaymentStatus::CanceledByClientOrBank
                | RapydPaymentStatus::Expired
                | RapydPaymentStatus::ReversedByRapyd,
                _,
            ) => Self::Voided,
            (RapydPaymentStatus::Error, _) => Self::Failure,
            (RapydPaymentStatus::New, _) => Self::Authorizing,
        }
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
    #[serde(rename = "not_applicable")]
    NotApplicable,
    #[serde(rename = "pending_confirmation")]
    PendingConfirmation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseData {
    pub id: String,
    pub amount: i64,
    pub status: RapydPaymentStatus,
    pub next_action: NextAction,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisputeResponseData {
    pub id: String,
    pub amount: i64,
    pub currency: api_models::enums::Currency,
    pub token: String,
    pub dispute_reason_description: String,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub due_date: Option<PrimitiveDateTime>,
    pub status: RapydWebhookDisputeStatus,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub updated_at: Option<PrimitiveDateTime>,
    pub original_transaction_id: String,
}

#[derive(Default, Debug, Serialize)]
pub struct RapydRefundRequest {
    pub payment: String,
    pub amount: Option<i64>,
    pub currency: Option<enums::Currency>,
}

impl<F> TryFrom<&RapydRouterData<&types::RefundsRouterData<F>>> for RapydRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RapydRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            payment: item
                .router_data
                .request
                .connector_transaction_id
                .to_string(),
            amount: Some(item.amount),
            currency: Some(item.router_data.request.currency),
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

impl TryFrom<&RapydRouterData<&types::PaymentsCaptureRouterData>> for CaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RapydRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: Some(item.amount),
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
                    diesel_models::enums::AttemptStatus::Failure => (
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
                        let redirction_url = data
                            .redirect_url
                            .as_ref()
                            .filter(|redirect_str| !redirect_str.is_empty())
                            .map(|url| {
                                Url::parse(url).into_report().change_context(
                                    errors::ConnectorError::FailedToObtainIntegrationUrl,
                                )
                            })
                            .transpose()?;

                        let redirection_data = redirction_url
                            .map(|url| services::RedirectForm::from((url, services::Method::Get)));

                        (
                            attempt_status,
                            Ok(types::PaymentsResponseData::TransactionResponse {
                                resource_id: types::ResponseId::ConnectorTransactionId(
                                    data.id.to_owned(),
                                ), //transaction_id is also the field but this id is used to initiate a refund
                                redirection_data,
                                mandate_reference: None,
                                connector_metadata: None,
                                network_txn_id: None,
                                connector_response_reference_id: None,
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
    PaymentDisputeCreated,
    PaymentDisputeUpdated,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, strum::Display)]
pub enum RapydWebhookDisputeStatus {
    #[serde(rename = "ACT")]
    Active,
    #[serde(rename = "RVW")]
    Review,
    #[serde(rename = "LOS")]
    Lose,
    #[serde(rename = "WIN")]
    Win,
    #[serde(other)]
    Unknown,
}

impl From<RapydWebhookDisputeStatus> for api_models::webhooks::IncomingWebhookEvent {
    fn from(value: RapydWebhookDisputeStatus) -> Self {
        match value {
            RapydWebhookDisputeStatus::Active => Self::DisputeOpened,
            RapydWebhookDisputeStatus::Review => Self::DisputeChallenged,
            RapydWebhookDisputeStatus::Lose => Self::DisputeLost,
            RapydWebhookDisputeStatus::Win => Self::DisputeWon,
            RapydWebhookDisputeStatus::Unknown => Self::EventNotSupported,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum WebhookData {
    Payment(ResponseData),
    Refund(RefundResponseData),
    Dispute(DisputeResponseData),
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
