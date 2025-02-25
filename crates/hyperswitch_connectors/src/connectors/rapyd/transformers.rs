use common_enums::enums;
use common_utils::{ext_traits::OptionExt, request::Method, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::{consts::NO_ERROR_CODE, errors};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, RouterData as _},
};

#[derive(Debug, Serialize)]
pub struct RapydRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for RapydRouterData<T> {
    fn from((amount, router_data): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct RapydPaymentsRequest {
    pub amount: MinorUnit,
    pub currency: enums::Currency,
    pub payment_method: PaymentMethod,
    pub payment_method_options: Option<PaymentMethodOptions>,
    pub merchant_reference_id: Option<String>,
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
    zip: Option<Secret<String>>,
    phone_number: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RapydWallet {
    #[serde(rename = "type")]
    payment_type: String,
    #[serde(rename = "details")]
    token: Option<Secret<String>>,
}

impl TryFrom<&RapydRouterData<&types::PaymentsAuthorizeRouterData>> for RapydPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RapydRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let (capture, payment_method_options) = match item.router_data.payment_method {
            enums::PaymentMethod::Card => {
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
                        Some(enums::CaptureMethod::Automatic)
                            | Some(enums::CaptureMethod::SequentialAutomatic)
                            | None
                    )),
                    Some(payment_method_options),
                )
            }
            _ => (None, None),
        };
        let payment_method = match item.router_data.request.payment_method_data {
            PaymentMethodData::Card(ref ccard) => {
                Some(PaymentMethod {
                    pm_type: "in_amex_card".to_owned(), //[#369] Map payment method type based on country
                    fields: Some(PaymentFields {
                        number: ccard.card_number.to_owned(),
                        expiration_month: ccard.card_exp_month.to_owned(),
                        expiration_year: ccard.card_exp_year.to_owned(),
                        name: item
                            .router_data
                            .get_optional_billing_full_name()
                            .to_owned()
                            .unwrap_or(Secret::new("".to_string())),
                        cvv: ccard.card_cvc.to_owned(),
                    }),
                    address: None,
                    digital_wallet: None,
                })
            }
            PaymentMethodData::Wallet(ref wallet_data) => {
                let digital_wallet = match wallet_data {
                    WalletData::GooglePay(data) => Some(RapydWallet {
                        payment_type: "google_pay".to_string(),
                        token: Some(Secret::new(data.tokenization_data.token.to_owned())),
                    }),
                    WalletData::ApplePay(data) => Some(RapydWallet {
                        payment_type: "apple_pay".to_string(),
                        token: Some(Secret::new(data.payment_data.to_string())),
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
        .get_required_value("payment_method not implemented")
        .change_context(errors::ConnectorError::NotImplemented(
            "payment_method".to_owned(),
        ))?;
        let return_url = item.router_data.request.get_router_return_url()?;
        Ok(Self {
            amount: item.amount,
            currency: item.router_data.request.currency,
            payment_method,
            capture,
            payment_method_options,
            merchant_reference_id: Some(item.router_data.connector_request_reference_id.clone()),
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

impl TryFrom<&ConnectorAuthType> for RapydAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
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

fn get_status(status: RapydPaymentStatus, next_action: NextAction) -> enums::AttemptStatus {
    match (status, next_action) {
        (RapydPaymentStatus::Closed, _) => enums::AttemptStatus::Charged,
        (
            RapydPaymentStatus::Active,
            NextAction::ThreedsVerification | NextAction::PendingConfirmation,
        ) => enums::AttemptStatus::AuthenticationPending,
        (RapydPaymentStatus::Active, NextAction::PendingCapture | NextAction::NotApplicable) => {
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
    pub merchant_reference_id: Option<String>,
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
    pub amount: Option<MinorUnit>,
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
    //Some field related to foreign exchange and split payment can be added as and when implemented
    pub id: String,
    pub payment: String,
    pub amount: i64,
    pub currency: enums::Currency,
    pub status: RefundStatus,
    pub created_at: Option<i64>,
    pub failure_reason: Option<String>,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let (connector_refund_id, refund_status) = match item.response.data {
            Some(data) => (data.id, enums::RefundStatus::from(data.status)),
            None => (
                item.response.status.error_code,
                enums::RefundStatus::Failure,
            ),
        };
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for types::RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let (connector_refund_id, refund_status) = match item.response.data {
            Some(data) => (data.id, enums::RefundStatus::from(data.status)),
            None => (
                item.response.status.error_code,
                enums::RefundStatus::Failure,
            ),
        };
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct CaptureRequest {
    amount: Option<MinorUnit>,
    receipt_email: Option<Secret<String>>,
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

impl<F, T> TryFrom<ResponseRouterData<F, RapydPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RapydPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (status, response) = match &item.response.data {
            Some(data) => {
                let attempt_status =
                    get_status(data.status.to_owned(), data.next_action.to_owned());
                match attempt_status {
                    enums::AttemptStatus::Failure => (
                        enums::AttemptStatus::Failure,
                        Err(ErrorResponse {
                            code: data
                                .failure_code
                                .to_owned()
                                .unwrap_or(item.response.status.error_code),
                            status_code: item.http_code,
                            message: item.response.status.status.unwrap_or_default(),
                            reason: data.failure_message.to_owned(),
                            attempt_status: None,
                            connector_transaction_id: None,
                        }),
                    ),
                    _ => {
                        let redirection_url = data
                            .redirect_url
                            .as_ref()
                            .filter(|redirect_str| !redirect_str.is_empty())
                            .map(|url| {
                                Url::parse(url).change_context(
                                    errors::ConnectorError::FailedToObtainIntegrationUrl,
                                )
                            })
                            .transpose()?;

                        let redirection_data =
                            redirection_url.map(|url| RedirectForm::from((url, Method::Get)));

                        (
                            attempt_status,
                            Ok(PaymentsResponseData::TransactionResponse {
                                resource_id: ResponseId::ConnectorTransactionId(data.id.to_owned()), //transaction_id is also the field but this id is used to initiate a refund
                                redirection_data: Box::new(redirection_data),
                                mandate_reference: Box::new(None),
                                connector_metadata: None,
                                network_txn_id: None,
                                connector_response_reference_id: data
                                    .merchant_reference_id
                                    .to_owned(),
                                incremental_authorization_allowed: None,
                                charges: None,
                            }),
                        )
                    }
                }
            }
            None => (
                enums::AttemptStatus::Failure,
                Err(ErrorResponse {
                    code: item.response.status.error_code,
                    status_code: item.http_code,
                    message: item.response.status.status.unwrap_or_default(),
                    reason: item.response.status.message,
                    attempt_status: None,
                    connector_transaction_id: None,
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
                error_code: NO_ERROR_CODE.to_owned(),
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
                error_code: NO_ERROR_CODE.to_owned(),
                status: None,
                message: None,
                response_code: None,
                operation_id: None,
            },
            data: Some(value),
        }
    }
}
