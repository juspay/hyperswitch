use common_utils::{errors::CustomResult, ext_traits::ByteSliceExt};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;

use crate::{
    connector::utils::{self, PaymentsCaptureRequestData, RouterData, WalletData},
    consts,
    core::errors,
    services,
    types::{self, api, storage::enums, transformers::ForeignFrom},
};

#[derive(Debug, Serialize)]
pub struct CheckoutRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for CheckoutRouterData<T>
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "token_data")]
pub enum TokenRequest {
    Googlepay(CheckoutGooglePayData),
    Applepay(CheckoutApplePayData),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "token_data")]
pub enum PreDecryptedTokenRequest {
    Applepay(Box<CheckoutApplePayData>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutGooglePayData {
    protocol_version: Secret<String>,
    signature: Secret<String>,
    signed_message: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckoutApplePayData {
    version: Secret<String>,
    data: Secret<String>,
    signature: Secret<String>,
    header: CheckoutApplePayHeader,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutApplePayHeader {
    ephemeral_public_key: Secret<String>,
    public_key_hash: Secret<String>,
    transaction_id: Secret<String>,
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
                api_models::payments::WalletData::AliPayQr(_)
                | api_models::payments::WalletData::AliPayRedirect(_)
                | api_models::payments::WalletData::AliPayHkRedirect(_)
                | api_models::payments::WalletData::MomoRedirect(_)
                | api_models::payments::WalletData::KakaoPayRedirect(_)
                | api_models::payments::WalletData::GoPayRedirect(_)
                | api_models::payments::WalletData::GcashRedirect(_)
                | api_models::payments::WalletData::ApplePayRedirect(_)
                | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
                | api_models::payments::WalletData::DanaRedirect {}
                | api_models::payments::WalletData::GooglePayRedirect(_)
                | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
                | api_models::payments::WalletData::MbWayRedirect(_)
                | api_models::payments::WalletData::MobilePayRedirect(_)
                | api_models::payments::WalletData::PaypalRedirect(_)
                | api_models::payments::WalletData::PaypalSdk(_)
                | api_models::payments::WalletData::SamsungPay(_)
                | api_models::payments::WalletData::TwintRedirect {}
                | api_models::payments::WalletData::VippsRedirect {}
                | api_models::payments::WalletData::TouchNGoRedirect(_)
                | api_models::payments::WalletData::WeChatPayRedirect(_)
                | api_models::payments::WalletData::CashappQr(_)
                | api_models::payments::WalletData::SwishQr(_)
                | api_models::payments::WalletData::WeChatPayQr(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("checkout"),
                    )
                    .into())
                }
            },
            api_models::payments::PaymentMethodData::Card(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::GiftCard(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("checkout"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct CheckoutTokenResponse {
    token: Secret<String>,
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
                token: item.response.token.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct CardSource {
    #[serde(rename = "type")]
    pub source_type: CheckoutSourceTypes,
    pub number: cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvv: Secret<String>,
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
    ApplePayPredecrypt(Box<ApplePayPredecrypt>),
}

#[derive(Debug, Serialize)]
pub struct ApplePayPredecrypt {
    token: Secret<String>,
    #[serde(rename = "type")]
    decrypt_type: String,
    token_type: String,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    eci: Option<Secret<String>>,
    cryptogram: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckoutSourceTypes {
    Card,
    Token,
}

pub struct CheckoutAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) processing_channel_id: Secret<String>,
    pub(super) api_secret: Secret<String>,
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
    pub processing_channel_id: Secret<String>,
    #[serde(rename = "3ds")]
    pub three_ds: CheckoutThreeDS,
    #[serde(flatten)]
    pub return_url: ReturnUrl,
    pub capture: bool,
    pub reference: String,
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
                api_key: api_key.to_owned(),
                api_secret: api_secret.to_owned(),
                processing_channel_id: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
impl TryFrom<&CheckoutRouterData<&types::PaymentsAuthorizeRouterData>> for PaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CheckoutRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let source_var = match item.router_data.request.payment_method_data.clone() {
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
                api_models::payments::WalletData::GooglePay(_) => {
                    Ok(PaymentSource::Wallets(WalletSource {
                        source_type: CheckoutSourceTypes::Token,
                        token: match item.router_data.get_payment_method_token()? {
                            types::PaymentMethodToken::Token(token) => token,
                            types::PaymentMethodToken::ApplePayDecrypt(_) => {
                                Err(errors::ConnectorError::InvalidWalletToken)?
                            }
                        },
                    }))
                }
                api_models::payments::WalletData::ApplePay(_) => {
                    let payment_method_token = item.router_data.get_payment_method_token()?;
                    match payment_method_token {
                        types::PaymentMethodToken::Token(apple_pay_payment_token) => {
                            Ok(PaymentSource::Wallets(WalletSource {
                                source_type: CheckoutSourceTypes::Token,
                                token: apple_pay_payment_token,
                            }))
                        }
                        types::PaymentMethodToken::ApplePayDecrypt(decrypt_data) => {
                            let expiry_year_4_digit = Secret::new(format!(
                                "20{}",
                                decrypt_data
                                    .clone()
                                    .application_expiration_date
                                    .peek()
                                    .get(0..2)
                                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                            ));
                            let exp_month = Secret::new(
                                decrypt_data
                                    .clone()
                                    .application_expiration_date
                                    .peek()
                                    .get(2..4)
                                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?
                                    .to_owned(),
                            );
                            Ok(PaymentSource::ApplePayPredecrypt(Box::new(
                                ApplePayPredecrypt {
                                    token: decrypt_data.application_primary_account_number,
                                    decrypt_type: "network_token".to_string(),
                                    token_type: "applepay".to_string(),
                                    expiry_month: exp_month,
                                    expiry_year: expiry_year_4_digit,
                                    eci: decrypt_data.payment_data.eci_indicator,
                                    cryptogram: decrypt_data.payment_data.online_payment_cryptogram,
                                },
                            )))
                        }
                    }
                }
                api_models::payments::WalletData::AliPayQr(_)
                | api_models::payments::WalletData::AliPayRedirect(_)
                | api_models::payments::WalletData::AliPayHkRedirect(_)
                | api_models::payments::WalletData::MomoRedirect(_)
                | api_models::payments::WalletData::KakaoPayRedirect(_)
                | api_models::payments::WalletData::GoPayRedirect(_)
                | api_models::payments::WalletData::GcashRedirect(_)
                | api_models::payments::WalletData::ApplePayRedirect(_)
                | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
                | api_models::payments::WalletData::DanaRedirect {}
                | api_models::payments::WalletData::GooglePayRedirect(_)
                | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
                | api_models::payments::WalletData::MbWayRedirect(_)
                | api_models::payments::WalletData::MobilePayRedirect(_)
                | api_models::payments::WalletData::PaypalRedirect(_)
                | api_models::payments::WalletData::PaypalSdk(_)
                | api_models::payments::WalletData::SamsungPay(_)
                | api_models::payments::WalletData::TwintRedirect {}
                | api_models::payments::WalletData::VippsRedirect {}
                | api_models::payments::WalletData::TouchNGoRedirect(_)
                | api_models::payments::WalletData::WeChatPayRedirect(_)
                | api_models::payments::WalletData::CashappQr(_)
                | api_models::payments::WalletData::SwishQr(_)
                | api_models::payments::WalletData::WeChatPayQr(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("checkout"),
                    ))
                }
            },

            api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Reward
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::GiftCard(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("checkout"),
                ))
            }
        }?;

        let three_ds = match item.router_data.auth_type {
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
                .router_data
                .request
                .router_return_url
                .as_ref()
                .map(|return_url| format!("{return_url}?status=success")),
            failure_url: item
                .router_data
                .request
                .router_return_url
                .as_ref()
                .map(|return_url| format!("{return_url}?status=failure")),
        };

        let capture = matches!(
            item.router_data.request.capture_method,
            Some(enums::CaptureMethod::Automatic)
        );

        let connector_auth = &item.router_data.connector_auth_type;
        let auth_type: CheckoutAuthType = connector_auth.try_into()?;
        let processing_channel_id = auth_type.processing_channel_id;
        Ok(Self {
            source: source_var,
            amount: item.amount.to_owned(),
            currency: item.router_data.request.currency.to_string(),
            processing_channel_id,
            three_ds,
            return_url,
            capture,
            reference: item.router_data.connector_request_reference_id.clone(),
        })
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum CheckoutPaymentStatus {
    Authorized,
    #[default]
    Pending,
    #[serde(rename = "Card Verified")]
    CardVerified,
    Declined,
    Captured,
}

impl TryFrom<CheckoutWebhookEventType> for CheckoutPaymentStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: CheckoutWebhookEventType) -> Result<Self, Self::Error> {
        match value {
            CheckoutWebhookEventType::PaymentApproved => Ok(Self::Authorized),
            CheckoutWebhookEventType::PaymentCaptured => Ok(Self::Captured),
            CheckoutWebhookEventType::PaymentDeclined => Ok(Self::Declined),
            CheckoutWebhookEventType::AuthenticationStarted
            | CheckoutWebhookEventType::AuthenticationApproved => Ok(Self::Pending),
            CheckoutWebhookEventType::PaymentRefunded
            | CheckoutWebhookEventType::PaymentRefundDeclined
            | CheckoutWebhookEventType::DisputeReceived
            | CheckoutWebhookEventType::DisputeExpired
            | CheckoutWebhookEventType::DisputeAccepted
            | CheckoutWebhookEventType::DisputeCanceled
            | CheckoutWebhookEventType::DisputeEvidenceSubmitted
            | CheckoutWebhookEventType::DisputeEvidenceAcknowledgedByScheme
            | CheckoutWebhookEventType::DisputeEvidenceRequired
            | CheckoutWebhookEventType::DisputeArbitrationLost
            | CheckoutWebhookEventType::DisputeArbitrationWon
            | CheckoutWebhookEventType::DisputeWon
            | CheckoutWebhookEventType::DisputeLost
            | CheckoutWebhookEventType::Unknown => {
                Err(errors::ConnectorError::WebhookEventTypeNotFound.into())
            }
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Href {
    #[serde(rename = "href")]
    redirection_url: Url,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct Links {
    redirect: Option<Href>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct PaymentsResponse {
    id: String,
    amount: Option<i32>,
    action_id: Option<String>,
    status: CheckoutPaymentStatus,
    #[serde(rename = "_links")]
    links: Links,
    balances: Option<Balances>,
    reference: Option<String>,
    response_code: Option<String>,
    response_summary: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PaymentsResponseEnum {
    ActionResponse(Vec<ActionResponse>),
    PaymentResponse(Box<PaymentsResponse>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
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
        let status = enums::AttemptStatus::foreign_from((
            item.response.status,
            item.data.request.capture_method,
        ));
        let error_response = if status == enums::AttemptStatus::Failure {
            Some(types::ErrorResponse {
                status_code: item.http_code,
                code: item
                    .response
                    .response_code
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .response_summary
                    .clone()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.response_summary,
            })
        } else {
            None
        };
        let payments_response_data = types::PaymentsResponseData::TransactionResponse {
            resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
            redirection_data,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(
                item.response.reference.unwrap_or(item.response.id),
            ),
        };
        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response_data), Err),
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
        let status =
            enums::AttemptStatus::foreign_from((item.response.status, item.response.balances));
        let error_response = if status == enums::AttemptStatus::Failure {
            Some(types::ErrorResponse {
                status_code: item.http_code,
                code: item
                    .response
                    .response_code
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .response_summary
                    .clone()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.response_summary,
            })
        } else {
            None
        };
        let payments_response_data = types::PaymentsResponseData::TransactionResponse {
            resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
            redirection_data,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(
                item.response.reference.unwrap_or(item.response.id),
            ),
        };
        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response_data), Err),
            ..item.data
        })
    }
}

impl TryFrom<types::PaymentsSyncResponseRouterData<PaymentsResponseEnum>>
    for types::PaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: types::PaymentsSyncResponseRouterData<PaymentsResponseEnum>,
    ) -> Result<Self, Self::Error> {
        let capture_sync_response_list = match item.response {
            PaymentsResponseEnum::PaymentResponse(payments_response) => {
                // for webhook consumption flow
                utils::construct_captures_response_hashmap(vec![payments_response])
            }
            PaymentsResponseEnum::ActionResponse(action_list) => {
                // for captures sync
                utils::construct_captures_response_hashmap(action_list)
            }
        };
        Ok(Self {
            response: Ok(types::PaymentsResponseData::MultipleCaptureResponse {
                capture_sync_response_list,
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
                connector_response_reference_id: None,
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
    pub processing_channel_id: Secret<String>,
    pub reference: Option<String>,
}

impl TryFrom<&CheckoutRouterData<&types::PaymentsCaptureRouterData>> for PaymentCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CheckoutRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let connector_auth = &item.router_data.connector_auth_type;
        let auth_type: CheckoutAuthType = connector_auth.try_into()?;
        let processing_channel_id = auth_type.processing_channel_id;
        let capture_type = if item.router_data.request.is_multiple_capture() {
            CaptureType::NonFinal
        } else {
            CaptureType::Final
        };
        let reference = item
            .router_data
            .request
            .multiple_capture_data
            .as_ref()
            .map(|multiple_capture_data| multiple_capture_data.capture_reference.clone());
        Ok(Self {
            amount: Some(item.amount.to_owned()),
            capture_type: Some(capture_type),
            processing_channel_id,
            reference, // hyperswitch's reference for this capture
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct PaymentCaptureResponse {
    pub action_id: String,
    pub reference: Option<String>,
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

        // if multiple capture request, return capture action_id so that it will be updated in the captures table.
        // else return previous connector_transaction_id.
        let resource_id = if item.data.request.is_multiple_capture() {
            item.response.action_id
        } else {
            item.data.request.connector_transaction_id.to_owned()
        };

        Ok(Self {
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(resource_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.reference,
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

impl<F> TryFrom<&CheckoutRouterData<&types::RefundsRouterData<F>>> for RefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CheckoutRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let reference = item.router_data.request.refund_id.clone();
        Ok(Self {
            amount: Some(item.amount.to_owned()),
            reference,
        })
    }
}
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
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
    pub error_type: Option<String>,
    pub error_codes: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, PartialEq)]
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

#[derive(Deserialize, Debug)]
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

impl utils::MultipleCaptureSyncResponse for ActionResponse {
    fn get_connector_capture_id(&self) -> String {
        self.action_id.clone()
    }

    fn get_capture_attempt_status(&self) -> enums::AttemptStatus {
        match self.approved {
            Some(true) => enums::AttemptStatus::Charged,
            Some(false) => enums::AttemptStatus::Failure,
            None => enums::AttemptStatus::Pending,
        }
    }

    fn get_connector_reference_id(&self) -> Option<String> {
        self.reference.clone()
    }

    fn is_capture_response(&self) -> bool {
        self.action_type == ActionType::Capture
    }

    fn get_amount_captured(&self) -> Option<i64> {
        Some(self.amount)
    }
}

impl utils::MultipleCaptureSyncResponse for Box<PaymentsResponse> {
    fn get_connector_capture_id(&self) -> String {
        self.action_id.clone().unwrap_or("".into())
    }

    fn get_capture_attempt_status(&self) -> enums::AttemptStatus {
        enums::AttemptStatus::foreign_from((self.status.clone(), self.balances.clone()))
    }

    fn get_connector_reference_id(&self) -> Option<String> {
        self.reference.clone()
    }

    fn is_capture_response(&self) -> bool {
        self.status == CheckoutPaymentStatus::Captured
    }
    fn get_amount_captured(&self) -> Option<i64> {
        match self.amount {
            Some(amount) => amount.try_into().ok(),
            None => None,
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

pub fn is_refund_event(event_code: &CheckoutWebhookEventType) -> bool {
    matches!(
        event_code,
        CheckoutWebhookEventType::PaymentRefunded | CheckoutWebhookEventType::PaymentRefundDeclined
    )
}

pub fn is_chargeback_event(event_code: &CheckoutWebhookEventType) -> bool {
    matches!(
        event_code,
        CheckoutWebhookEventType::DisputeReceived
            | CheckoutWebhookEventType::DisputeExpired
            | CheckoutWebhookEventType::DisputeAccepted
            | CheckoutWebhookEventType::DisputeCanceled
            | CheckoutWebhookEventType::DisputeEvidenceSubmitted
            | CheckoutWebhookEventType::DisputeEvidenceAcknowledgedByScheme
            | CheckoutWebhookEventType::DisputeEvidenceRequired
            | CheckoutWebhookEventType::DisputeArbitrationLost
            | CheckoutWebhookEventType::DisputeArbitrationWon
            | CheckoutWebhookEventType::DisputeWon
            | CheckoutWebhookEventType::DisputeLost
    )
}

#[derive(Debug, Deserialize, strum::Display, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutWebhookEventType {
    AuthenticationStarted,
    AuthenticationApproved,
    PaymentApproved,
    PaymentCaptured,
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
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookEventTypeBody {
    #[serde(rename = "type")]
    pub transaction_type: CheckoutWebhookEventType,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookData {
    pub id: String,
    pub payment_id: Option<String>,
    pub action_id: Option<String>,
    pub reference: Option<String>,
    pub amount: i32,
    pub balances: Option<Balances>,
    pub response_code: Option<String>,
    pub response_summary: Option<String>,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookBody {
    #[serde(rename = "type")]
    pub transaction_type: CheckoutWebhookEventType,
    pub data: CheckoutWebhookData,
    #[serde(rename = "_links")]
    pub links: Links,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutDisputeWebhookData {
    pub id: String,
    pub payment_id: Option<String>,
    pub action_id: Option<String>,
    pub amount: i32,
    pub currency: String,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub evidence_required_by: Option<PrimitiveDateTime>,
    pub reason_code: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub date: Option<PrimitiveDateTime>,
}
#[derive(Debug, Deserialize)]
pub struct CheckoutDisputeWebhookBody {
    #[serde(rename = "type")]
    pub transaction_type: CheckoutTransactionType,
    pub data: CheckoutDisputeWebhookData,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_on: Option<PrimitiveDateTime>,
}
#[derive(Debug, Deserialize, strum::Display, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutTransactionType {
    AuthenticationStarted,
    AuthenticationApproved,
    PaymentApproved,
    PaymentCaptured,
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

impl From<CheckoutWebhookEventType> for api::IncomingWebhookEvent {
    fn from(transaction_type: CheckoutWebhookEventType) -> Self {
        match transaction_type {
            CheckoutWebhookEventType::AuthenticationStarted => Self::EventNotSupported,
            CheckoutWebhookEventType::AuthenticationApproved => Self::EventNotSupported,
            CheckoutWebhookEventType::PaymentApproved => Self::EventNotSupported,
            CheckoutWebhookEventType::PaymentCaptured => Self::PaymentIntentSuccess,
            CheckoutWebhookEventType::PaymentDeclined => Self::PaymentIntentFailure,
            CheckoutWebhookEventType::PaymentRefunded => Self::RefundSuccess,
            CheckoutWebhookEventType::PaymentRefundDeclined => Self::RefundFailure,
            CheckoutWebhookEventType::DisputeReceived
            | CheckoutWebhookEventType::DisputeEvidenceRequired => Self::DisputeOpened,
            CheckoutWebhookEventType::DisputeExpired => Self::DisputeExpired,
            CheckoutWebhookEventType::DisputeAccepted => Self::DisputeAccepted,
            CheckoutWebhookEventType::DisputeCanceled => Self::DisputeCancelled,
            CheckoutWebhookEventType::DisputeEvidenceSubmitted
            | CheckoutWebhookEventType::DisputeEvidenceAcknowledgedByScheme => {
                Self::DisputeChallenged
            }
            CheckoutWebhookEventType::DisputeWon
            | CheckoutWebhookEventType::DisputeArbitrationWon => Self::DisputeWon,
            CheckoutWebhookEventType::DisputeLost
            | CheckoutWebhookEventType::DisputeArbitrationLost => Self::DisputeLost,
            CheckoutWebhookEventType::Unknown => Self::EventNotSupported,
        }
    }
}

impl From<CheckoutTransactionType> for api_models::enums::DisputeStage {
    fn from(code: CheckoutTransactionType) -> Self {
        match code {
            CheckoutTransactionType::DisputeArbitrationLost
            | CheckoutTransactionType::DisputeArbitrationWon => Self::PreArbitration,
            _ => Self::Dispute,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookObjectResource {
    pub data: serde_json::Value,
}

pub fn construct_file_upload_request(
    file_upload_router_data: types::UploadFileRouterData,
) -> CustomResult<reqwest::multipart::Form, errors::ConnectorError> {
    let request = file_upload_router_data.request;
    let mut multipart = reqwest::multipart::Form::new();
    multipart = multipart.text("purpose", "dispute_evidence");
    let file_data = reqwest::multipart::Part::bytes(request.file)
        .file_name(format!(
            "{}.{}",
            request.file_key,
            request
                .file_type
                .to_string()
                .split('/')
                .last()
                .unwrap_or_default()
        ))
        .mime_str(request.file_type.as_ref())
        .into_report()
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failure in constructing file data")?;
    multipart = multipart.part("file", file_data);
    Ok(multipart)
}

#[derive(Debug, Deserialize)]
pub struct FileUploadResponse {
    #[serde(rename = "id")]
    pub file_id: String,
}

#[derive(Default, Debug, Serialize)]
pub struct Evidence {
    pub proof_of_delivery_or_service_file: Option<String>,
    pub invoice_or_receipt_file: Option<String>,
    pub invoice_showing_distinct_transactions_file: Option<String>,
    pub customer_communication_file: Option<String>,
    pub refund_or_cancellation_policy_file: Option<String>,
    pub recurring_transaction_agreement_file: Option<String>,
    pub additional_evidence_file: Option<String>,
}

impl TryFrom<&api::IncomingWebhookRequestDetails<'_>> for PaymentsResponse {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(request: &api::IncomingWebhookRequestDetails<'_>) -> Result<Self, Self::Error> {
        let details: CheckoutWebhookBody = request
            .body
            .parse_struct("CheckoutWebhookBody")
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        let data = details.data;
        let psync_struct = Self {
            id: data.payment_id.unwrap_or(data.id),
            amount: Some(data.amount),
            status: CheckoutPaymentStatus::try_from(details.transaction_type)?,
            links: details.links,
            balances: data.balances,
            reference: data.reference,
            response_code: data.response_code,
            response_summary: data.response_summary,
            action_id: data.action_id,
        };

        Ok(psync_struct)
    }
}

impl TryFrom<&types::SubmitEvidenceRouterData> for Evidence {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::SubmitEvidenceRouterData) -> Result<Self, Self::Error> {
        let submit_evidence_request_data = item.request.clone();
        Ok(Self {
            proof_of_delivery_or_service_file: submit_evidence_request_data
                .shipping_documentation_provider_file_id,
            invoice_or_receipt_file: submit_evidence_request_data.receipt_provider_file_id,
            invoice_showing_distinct_transactions_file: submit_evidence_request_data
                .invoice_showing_distinct_transactions_provider_file_id,
            customer_communication_file: submit_evidence_request_data
                .customer_communication_provider_file_id,
            refund_or_cancellation_policy_file: submit_evidence_request_data
                .refund_policy_provider_file_id,
            recurring_transaction_agreement_file: submit_evidence_request_data
                .recurring_transaction_agreement_provider_file_id,
            additional_evidence_file: submit_evidence_request_data
                .uncategorized_file_provider_file_id,
        })
    }
}

impl From<String> for utils::ErrorCodeAndMessage {
    fn from(error: String) -> Self {
        Self {
            error_code: error.clone(),
            error_message: error,
        }
    }
}
