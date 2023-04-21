use error_stack::IntoReport;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::{RouterData, WalletData},
    core::errors,
    pii, services,
    types::{self, api, storage::enums, transformers::ForeignFrom},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "token_data")]
pub enum TokenRequest {
    Googlepay(CheckoutGooglePayData),
    Applepay(CheckoutApplePayData),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutGooglePayData {
    protocol_version: pii::Secret<String>,
    signature: pii::Secret<String>,
    signed_message: pii::Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckoutApplePayData {
    version: pii::Secret<String>,
    data: pii::Secret<String>,
    signature: pii::Secret<String>,
    header: CheckoutApplePayHeader,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutApplePayHeader {
    ephemeral_public_key: pii::Secret<String>,
    public_key_hash: pii::Secret<String>,
    transaction_id: pii::Secret<String>,
}

impl TryFrom<&types::TokenizationRouterData> for TokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Wallet(wallet_data) => match wallet_data.clone() {
                api_models::payments::WalletData::GooglePay(_data) => {
                    let json_wallet_data: CheckoutGooglePayData =
                        wallet_data.get_wallet_token_as_json()?;
                    Ok(Self::Googlepay(json_wallet_data))
                }
                api_models::payments::WalletData::ApplePay(_data) => {
                    let json_wallet_data: CheckoutApplePayData =
                        wallet_data.get_wallet_token_as_json()?;
                    Ok(Self::Applepay(json_wallet_data))
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    "Payment Method".to_string(),
                ))
                .into_report(),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))
            .into_report(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct CheckoutTokenResponse {
    token: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, CheckoutTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, CheckoutTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TokenizationResponse {
                token: item.response.token,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct CardSource {
    #[serde(rename = "type")]
    pub source_type: CheckoutSourceTypes,
    pub number: pii::Secret<String, pii::CardNumber>,
    pub expiry_month: pii::Secret<String>,
    pub expiry_year: pii::Secret<String>,
    pub cvv: pii::Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct WalletSource {
    #[serde(rename = "type")]
    pub source_type: CheckoutSourceTypes,
    pub token: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentSource {
    Card(CardSource),
    Wallets(WalletSource),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckoutSourceTypes {
    Card,
    Token,
}

pub struct CheckoutAuthType {
    pub(super) api_key: String,
    pub(super) processing_channel_id: String,
    pub(super) api_secret: String,
}

#[derive(Debug, Serialize)]
pub struct ReturnUrl {
    pub success_url: Option<String>,
    pub failure_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentsRequest {
    pub source: PaymentSource,
    pub amount: i64,
    pub currency: String,
    pub processing_channel_id: String,
    #[serde(rename = "3ds")]
    pub three_ds: CheckoutThreeDS,
    #[serde(flatten)]
    pub return_url: ReturnUrl,
    pub capture: bool,
}

#[derive(Debug, Serialize)]
pub struct CheckoutThreeDS {
    enabled: bool,
    force_3ds: bool,
}

impl TryFrom<&types::ConnectorAuthType> for CheckoutAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            api_secret,
            key1,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_string(),
                api_secret: api_secret.to_string(),
                processing_channel_id: key1.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
impl TryFrom<&types::PaymentsAuthorizeRouterData> for PaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let source_var = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => {
                let a = PaymentSource::Card(CardSource {
                    source_type: CheckoutSourceTypes::Card,
                    number: ccard.card_number.clone(),
                    expiry_month: ccard.card_exp_month.clone(),
                    expiry_year: ccard.card_exp_year.clone(),
                    cvv: ccard.card_cvc,
                });
                Ok(a)
            }
            api::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                api_models::payments::WalletData::GooglePay(_)
                | api_models::payments::WalletData::ApplePay(_) => {
                    Ok(PaymentSource::Wallets(WalletSource {
                        source_type: CheckoutSourceTypes::Token,
                        token: item.get_payment_method_token()?,
                    }))
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    "Payment Method".to_string(),
                )),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            )),
        }?;

        let three_ds = match item.auth_type {
            enums::AuthenticationType::ThreeDs => CheckoutThreeDS {
                enabled: true,
                force_3ds: true,
            },
            enums::AuthenticationType::NoThreeDs => CheckoutThreeDS {
                enabled: false,
                force_3ds: false,
            },
        };

        let return_url = ReturnUrl {
            success_url: item
                .request
                .router_return_url
                .as_ref()
                .map(|return_url| format!("{return_url}?status=success")),
            failure_url: item
                .request
                .router_return_url
                .as_ref()
                .map(|return_url| format!("{return_url}?status=failure")),
        };

        let capture = matches!(
            item.request.capture_method,
            Some(enums::CaptureMethod::Automatic)
        );

        let connector_auth = &item.connector_auth_type;
        let auth_type: CheckoutAuthType = connector_auth.try_into()?;
        let processing_channel_id = auth_type.processing_channel_id;
        Ok(Self {
            source: source_var,
            amount: item.request.amount,
            currency: item.request.currency.to_string(),
            processing_channel_id,
            three_ds,
            return_url,
            capture,
        })
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Deserialize)]
pub enum CheckoutPaymentStatus {
    Authorized,
    #[default]
    Pending,
    #[serde(rename = "Card Verified")]
    CardVerified,
    Declined,
    Captured,
}

impl ForeignFrom<(CheckoutPaymentStatus, Option<enums::CaptureMethod>)> for enums::AttemptStatus {
    fn foreign_from(item: (CheckoutPaymentStatus, Option<enums::CaptureMethod>)) -> Self {
        let (status, capture_method) = item;
        match status {
            CheckoutPaymentStatus::Authorized => {
                if capture_method == Some(enums::CaptureMethod::Automatic)
                    || capture_method.is_none()
                {
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            CheckoutPaymentStatus::Captured => Self::Charged,
            CheckoutPaymentStatus::Declined => Self::Failure,
            CheckoutPaymentStatus::Pending => Self::AuthenticationPending,
            CheckoutPaymentStatus::CardVerified => Self::Pending,
        }
    }
}

impl ForeignFrom<(CheckoutPaymentStatus, Option<Balances>)> for enums::AttemptStatus {
    fn foreign_from(item: (CheckoutPaymentStatus, Option<Balances>)) -> Self {
        let (status, balances) = item;

        match status {
            CheckoutPaymentStatus::Authorized => {
                if let Some(Balances {
                    available_to_capture: 0,
                }) = balances
                {
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            CheckoutPaymentStatus::Captured => Self::Charged,
            CheckoutPaymentStatus::Declined => Self::Failure,
            CheckoutPaymentStatus::Pending => Self::AuthenticationPending,
            CheckoutPaymentStatus::CardVerified => Self::Pending,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct Href {
    #[serde(rename = "href")]
    redirection_url: Url,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct Links {
    redirect: Option<Href>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct PaymentsResponse {
    id: String,
    amount: Option<i32>,
    status: CheckoutPaymentStatus,
    #[serde(rename = "_links")]
    links: Links,
    balances: Option<Balances>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct Balances {
    available_to_capture: i32,
}

impl TryFrom<types::PaymentsResponseRouterData<PaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<PaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item.response.links.redirect.map(|href| {
            services::RedirectForm::from((href.redirection_url, services::Method::Get))
        });

        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.status,
                item.data.request.capture_method,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::PaymentsSyncResponseRouterData<PaymentsResponse>>
    for types::PaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsSyncResponseRouterData<PaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item.response.links.redirect.map(|href| {
            services::RedirectForm::from((href.redirection_url, services::Method::Get))
        });

        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.status,
                item.response.balances,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize)]
pub struct PaymentVoidRequest {
    reference: String,
}
#[derive(Clone, Default, Debug, Eq, PartialEq, Deserialize)]
pub struct PaymentVoidResponse {
    #[serde(skip)]
    pub(super) status: u16,
    action_id: String,
    reference: String,
}

impl From<&PaymentVoidResponse> for enums::AttemptStatus {
    fn from(item: &PaymentVoidResponse) -> Self {
        if item.status == 202 {
            Self::Voided
        } else {
            Self::VoidFailed
        }
    }
}

impl TryFrom<types::PaymentsCancelResponseRouterData<PaymentVoidResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsCancelResponseRouterData<PaymentVoidResponse>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(response.action_id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            status: response.into(),
            ..item.data
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for PaymentVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            reference: item.request.connector_transaction_id.clone(),
        })
    }
}

#[derive(Debug, Serialize)]
pub enum CaptureType {
    Final,
    NonFinal,
}

#[derive(Debug, Serialize)]
pub struct PaymentCaptureRequest {
    pub amount: Option<i64>,
    pub capture_type: Option<CaptureType>,
    pub processing_channel_id: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for PaymentCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let connector_auth = &item.connector_auth_type;
        let auth_type: CheckoutAuthType = connector_auth.try_into()?;
        let processing_channel_id = auth_type.processing_channel_id;
        Ok(Self {
            amount: Some(item.request.amount_to_capture),
            capture_type: Some(CaptureType::Final),
            processing_channel_id,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct PaymentCaptureResponse {
    pub action_id: String,
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<PaymentCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<PaymentCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let (status, amount_captured) = if item.http_code == 202 {
            (
                enums::AttemptStatus::Charged,
                Some(item.data.request.amount_to_capture),
            )
        } else {
            (enums::AttemptStatus::Pending, None)
        };
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.data.request.connector_transaction_id.to_owned(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            status,
            amount_captured,
            ..item.data
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefundRequest {
    amount: Option<i64>,
    reference: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let amount = item.request.refund_amount;
        let reference = item.request.refund_id.clone();
        Ok(Self {
            amount: Some(amount),
            reference,
        })
    }
}
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct RefundResponse {
    action_id: String,
    reference: String,
}

#[derive(Deserialize)]
pub struct CheckoutRefundResponse {
    pub(super) status: u16,
    pub(super) response: RefundResponse,
}

impl From<&CheckoutRefundResponse> for enums::RefundStatus {
    fn from(item: &CheckoutRefundResponse) -> Self {
        if item.status == 202 {
            Self::Success
        } else {
            Self::Failure
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, CheckoutRefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, CheckoutRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(&item.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, CheckoutRefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, CheckoutRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(&item.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct ErrorResponse {
    pub request_id: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub error_codes: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub enum ActionType {
    Authorization,
    Void,
    Capture,
    Refund,
    Payout,
    Return,
    #[serde(rename = "Card Verification")]
    CardVerification,
}

#[derive(Deserialize)]
pub struct ActionResponse {
    #[serde(rename = "id")]
    pub action_id: String,
    pub amount: i64,
    #[serde(rename = "type")]
    pub action_type: ActionType,
    pub approved: Option<bool>,
    pub reference: Option<String>,
}

impl From<&ActionResponse> for enums::RefundStatus {
    fn from(item: &ActionResponse) -> Self {
        match item.approved {
            Some(true) => Self::Success,
            Some(false) => Self::Failure,
            None => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckoutRedirectResponseStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone, serde::Deserialize, Eq, PartialEq)]
pub struct CheckoutRedirectResponse {
    pub status: Option<CheckoutRedirectResponseStatus>,
    #[serde(rename = "cko-session-id")]
    pub cko_session_id: Option<String>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, &ActionResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, &ActionResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, &ActionResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, &ActionResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<CheckoutRedirectResponseStatus> for enums::AttemptStatus {
    fn from(item: CheckoutRedirectResponseStatus) -> Self {
        match item {
            CheckoutRedirectResponseStatus::Success => Self::AuthenticationSuccessful,

            CheckoutRedirectResponseStatus::Failure => Self::Failure,
        }
    }
}

pub fn is_refund_event(event_code: &CheckoutTxnType) -> bool {
    matches!(
        event_code,
        CheckoutTxnType::PaymentRefunded | CheckoutTxnType::PaymentRefundDeclined
    )
}

pub fn is_chargeback_event(event_code: &CheckoutTxnType) -> bool {
    matches!(
        event_code,
        CheckoutTxnType::DisputeReceived
            | CheckoutTxnType::DisputeExpired
            | CheckoutTxnType::DisputeAccepted
            | CheckoutTxnType::DisputeCanceled
            | CheckoutTxnType::DisputeEvidenceSubmitted
            | CheckoutTxnType::DisputeEvidenceAcknowledgedByScheme
            | CheckoutTxnType::DisputeEvidenceRequired
            | CheckoutTxnType::DisputeArbitrationLost
            | CheckoutTxnType::DisputeArbitrationWon
            | CheckoutTxnType::DisputeWon
            | CheckoutTxnType::DisputeLost
    )
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookData {
    pub id: String,
    pub payment_id: Option<String>,
    pub action_id: Option<String>,
    pub amount: i32,
    pub currency: String,
    pub evidence_required_by: Option<String>,
    pub reason_code: Option<String>,
    pub date: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookBody {
    #[serde(rename = "type")]
    pub txn_type: CheckoutTxnType,
    pub data: CheckoutWebhookData,
    pub created_on: Option<String>,
}
#[derive(Debug, Deserialize, strum::Display, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutTxnType {
    PaymentApproved,
    PaymentDeclined,
    PaymentRefunded,
    PaymentRefundDeclined,
    DisputeReceived,
    DisputeExpired,
    DisputeAccepted,
    DisputeCanceled,
    DisputeEvidenceSubmitted,
    DisputeEvidenceAcknowledgedByScheme,
    DisputeEvidenceRequired,
    DisputeArbitrationLost,
    DisputeArbitrationWon,
    DisputeWon,
    DisputeLost,
}

impl From<CheckoutTxnType> for api::IncomingWebhookEvent {
    fn from(txn_type: CheckoutTxnType) -> Self {
        match txn_type {
            CheckoutTxnType::PaymentApproved => Self::PaymentIntentSuccess,
            CheckoutTxnType::PaymentDeclined => Self::PaymentIntentSuccess,
            CheckoutTxnType::PaymentRefunded => Self::RefundSuccess,
            CheckoutTxnType::PaymentRefundDeclined => Self::RefundFailure,
            CheckoutTxnType::DisputeReceived | CheckoutTxnType::DisputeEvidenceRequired => {
                Self::DisputeOpened
            }
            CheckoutTxnType::DisputeExpired => Self::DisputeExpired,
            CheckoutTxnType::DisputeAccepted => Self::DisputeAccepted,
            CheckoutTxnType::DisputeCanceled => Self::DisputeCancelled,
            CheckoutTxnType::DisputeEvidenceSubmitted
            | CheckoutTxnType::DisputeEvidenceAcknowledgedByScheme => Self::DisputeChallenged,
            CheckoutTxnType::DisputeWon | CheckoutTxnType::DisputeArbitrationWon => {
                Self::DisputeWon
            }
            CheckoutTxnType::DisputeLost | CheckoutTxnType::DisputeArbitrationLost => {
                Self::DisputeLost
            }
        }
    }
}

impl From<CheckoutTxnType> for api_models::enums::DisputeStage {
    fn from(code: CheckoutTxnType) -> Self {
        match code {
            CheckoutTxnType::DisputeArbitrationLost | CheckoutTxnType::DisputeArbitrationWon => {
                Self::PreArbitration
            }
            _ => Self::Dispute,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookObjectResource {
    pub data: serde_json::Value,
}
