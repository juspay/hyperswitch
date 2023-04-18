use common_utils::{
    crypto::{self, GenerateDigest},
    date_time,
    pii::Email,
};
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, PaymentsAuthorizeRequestData, PaymentsCancelRequestData, RouterData},
    consts,
    core::errors,
    services,
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct NuveiMeta {
    pub session_token: String,
}

#[derive(Debug, Serialize, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiSessionRequest {
    pub merchant_id: String,
    pub merchant_site_id: String,
    pub client_request_id: String,
    pub time_stamp: String,
    pub checksum: String,
}

#[derive(Debug, Serialize, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiSessionResponse {
    pub session_token: String,
    pub internal_request_id: i64,
    pub status: String,
    pub err_code: i64,
    pub reason: String,
    pub merchant_id: String,
    pub merchant_site_id: String,
    pub version: String,
    pub client_request_id: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentsRequest {
    pub time_stamp: String,
    pub session_token: String,
    pub merchant_id: String,
    pub merchant_site_id: String,
    pub client_request_id: String,
    pub amount: String,
    pub currency: String,
    pub user_token_id: String,
    pub client_unique_id: String,
    pub transaction_type: TransactionType,
    pub payment_option: PaymentOption,
    pub checksum: String,
    pub billing_address: Option<BillingAddress>,
    pub related_transaction_id: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct NuveiInitPaymentRequest {
    pub session_token: String,
    pub merchant_id: String,
    pub merchant_site_id: String,
    pub client_request_id: String,
    pub amount: String,
    pub currency: String,
    pub payment_option: PaymentOption,
    pub checksum: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentFlowRequest {
    pub time_stamp: String,
    pub merchant_id: String,
    pub merchant_site_id: String,
    pub client_request_id: String,
    pub amount: String,
    pub currency: String,
    pub related_transaction_id: Option<String>,
    pub checksum: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentSyncRequest {
    pub session_token: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum TransactionType {
    Auth,
    #[default]
    Sale,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentOption {
    pub card: Option<Card>,
    pub redirect_url: Option<Url>,
    pub user_payment_option_id: Option<String>,
    pub device_details: Option<DeviceDetails>,
    pub alternative_payment_method: Option<AlternativePaymentMethod>,
    pub billing_address: Option<BillingAddress>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternativePaymentMethod {
    pub payment_method: AlternativePaymentMethodType,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlternativePaymentMethodType {
    #[default]
    ApmgwExpresscheckout,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BillingAddress {
    pub email: Secret<String, Email>,
    pub country: api_models::enums::CountryCode,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub card_number: Option<Secret<String, common_utils::pii::CardNumber>>,
    pub card_holder_name: Option<Secret<String>>,
    pub expiration_month: Option<Secret<String>>,
    pub expiration_year: Option<Secret<String>>,
    #[serde(rename = "CVV")]
    pub cvv: Option<Secret<String>>,
    pub three_d: Option<ThreeD>,
    pub cc_card_number: Option<String>,
    pub bin: Option<String>,
    pub last4_digits: Option<String>,
    pub cc_exp_month: Option<String>,
    pub cc_exp_year: Option<String>,
    pub acquirer_id: Option<String>,
    pub cvv2_reply: Option<String>,
    pub avs_code: Option<String>,
    pub card_type: Option<String>,
    pub card_brand: Option<String>,
    pub issuer_bank_name: Option<String>,
    pub issuer_country: Option<String>,
    pub is_prepaid: Option<String>,
    pub external_token: Option<ExternalToken>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalToken {
    pub external_token_provider: ExternalTokenProvider,
    pub mobile_token: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ExternalTokenProvider {
    #[default]
    GooglePay,
    ApplePay,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeD {
    pub method_completion_ind: Option<MethodCompletion>,
    pub browser_details: Option<BrowserDetails>,
    pub version: Option<String>,
    #[serde(rename = "notificationURL")]
    pub notification_url: Option<String>,
    #[serde(rename = "merchantURL")]
    pub merchant_url: Option<String>,
    pub acs_url: Option<String>,
    pub c_req: Option<String>,
    pub platform_type: Option<PlatformType>,
    pub v2supported: Option<String>,
    pub v2_additional_params: Option<V2AdditionalParams>,
    pub is_liability_on_issuer: Option<LiabilityShift>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum MethodCompletion {
    #[serde(rename = "Y")]
    Success,
    #[serde(rename = "N")]
    Failure,
    #[serde(rename = "U")]
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum PlatformType {
    #[serde(rename = "01")]
    App,
    #[serde(rename = "02")]
    #[default]
    Browser,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserDetails {
    pub accept_header: String,
    pub ip: Option<std::net::IpAddr>,
    pub java_enabled: String,
    pub java_script_enabled: String,
    pub language: String,
    pub color_depth: u8,
    pub screen_height: u32,
    pub screen_width: u32,
    pub time_zone: i32,
    pub user_agent: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AdditionalParams {
    pub challenge_window_size: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDetails {
    pub ip_address: String,
}

impl From<enums::CaptureMethod> for TransactionType {
    fn from(value: enums::CaptureMethod) -> Self {
        match value {
            enums::CaptureMethod::Manual => Self::Auth,
            _ => Self::Sale,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NuveiRedirectionResponse {
    pub cres: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiACSResponse {
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: String,
    #[serde(rename = "acsTransID")]
    pub acs_trans_id: String,
    pub message_type: String,
    pub message_version: String,
    pub trans_status: Option<LiabilityShift>,
    pub message_extension: Vec<MessageExtension>,
    pub acs_signed_content: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageExtension {
    pub name: String,
    pub id: String,
    pub criticality_indicator: bool,
    pub data: MessageExtensionData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageExtensionData {
    pub value_one: String,
    pub value_two: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiabilityShift {
    #[serde(rename = "Y", alias = "1")]
    Success,
    #[serde(rename = "N", alias = "0")]
    Failed,
}

fn encode_payload(
    payload: Vec<String>,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let data = payload.join("");
    let digest = crypto::Sha256
        .generate_digest(data.as_bytes())
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("error encoding the payload")?;
    Ok(hex::encode(digest))
}

impl TryFrom<&types::PaymentsAuthorizeSessionTokenRouterData> for NuveiSessionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &types::PaymentsAuthorizeSessionTokenRouterData,
    ) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.attempt_id.clone();
        let time_stamp = date_time::date_as_yyyymmddhhmmss()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_secret = connector_meta.merchant_secret;
        Ok(Self {
            merchant_id: merchant_id.clone(),
            merchant_site_id: merchant_site_id.clone(),
            client_request_id: client_request_id.clone(),
            time_stamp: time_stamp.clone(),
            checksum: encode_payload(vec![
                merchant_id,
                merchant_site_id,
                client_request_id,
                time_stamp,
                merchant_secret,
            ])?,
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, NuveiSessionResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, NuveiSessionResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Pending,
            session_token: Some(item.response.session_token.clone()),
            response: Ok(types::PaymentsResponseData::SessionTokenResponse {
                session_token: item.response.session_token,
            }),
            ..item.data
        })
    }
}

impl<F>
    TryFrom<(
        &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
        String,
    )> for NuveiPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
            String,
        ),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let session_token = data.1;
        if session_token.is_empty() {
            return Err(errors::ConnectorError::MissingRequiredField {
                field_name: "session_token",
            }
            .into());
        }
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.attempt_id.clone();
        let time_stamp = date_time::date_as_yyyymmddhhmmss()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_secret = connector_meta.merchant_secret;
        let request_data = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(card) => get_card_info(item, &card),
            api::PaymentMethodData::Wallet(wallet) => match wallet {
                api_models::payments::WalletData::GooglePay(gpay_data) => Ok(Self {
                    payment_option: PaymentOption {
                        card: Some(Card {
                            external_token: Some(ExternalToken {
                                external_token_provider: ExternalTokenProvider::GooglePay,
                                mobile_token: gpay_data.tokenization_data.token,
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                api_models::payments::WalletData::ApplePay(apple_data) => Ok(Self {
                    payment_option: PaymentOption {
                        card: Some(Card {
                            external_token: Some(ExternalToken {
                                external_token_provider: ExternalTokenProvider::ApplePay,
                                mobile_token: apple_data.payment_data,
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                api_models::payments::WalletData::PaypalRedirect(_) => Ok(Self {
                    payment_option: PaymentOption {
                        alternative_payment_method: Some(AlternativePaymentMethod {
                            payment_method: AlternativePaymentMethodType::ApmgwExpresscheckout,
                        }),
                        ..Default::default()
                    },
                    billing_address: Some(BillingAddress {
                        email: item.request.get_email()?,
                        country: item.get_billing_country()?,
                    }),
                    ..Default::default()
                }),
                _ => Err(errors::ConnectorError::NotSupported {
                    payment_method: "Wallet".to_string(),
                    connector: "Nuvei",
                    payment_experience: "RedirectToUrl".to_string(),
                }
                .into()),
            },
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }?;
        Ok(Self {
            merchant_id: merchant_id.clone(),
            merchant_site_id: merchant_site_id.clone(),
            client_request_id: client_request_id.clone(),
            amount: item.request.amount.clone().to_string(),
            currency: item.request.currency.clone().to_string(),
            transaction_type: item
                .request
                .capture_method
                .map(TransactionType::from)
                .unwrap_or_default(),
            time_stamp: time_stamp.clone(),
            session_token,
            checksum: encode_payload(vec![
                merchant_id,
                merchant_site_id,
                client_request_id,
                item.request.amount.to_string(),
                item.request.currency.to_string(),
                time_stamp,
                merchant_secret,
            ])?,
            ..request_data
        })
    }
}
fn get_card_info<F>(
    item: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    card_details: &api_models::payments::Card,
) -> Result<NuveiPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let browser_info = item.request.get_browser_info()?;
    let related_transaction_id = if item.request.enrolled_for_3ds {
        item.request.related_transaction_id.clone()
    } else {
        None
    };
    let three_d = if item.request.enrolled_for_3ds {
        Some(ThreeD {
            browser_details: Some(BrowserDetails {
                accept_header: browser_info.accept_header,
                ip: browser_info.ip_address,
                java_enabled: browser_info.java_enabled.to_string().to_uppercase(),
                java_script_enabled: browser_info.java_script_enabled.to_string().to_uppercase(),
                language: browser_info.language,
                color_depth: browser_info.color_depth,
                screen_height: browser_info.screen_height,
                screen_width: browser_info.screen_width,
                time_zone: browser_info.time_zone,
                user_agent: browser_info.user_agent,
            }),
            notification_url: item.request.complete_authorize_url.clone(),
            merchant_url: item.return_url.clone(),
            platform_type: Some(PlatformType::Browser),
            method_completion_ind: Some(MethodCompletion::Unavailable),
            ..Default::default()
        })
    } else {
        None
    };
    let card = card_details.clone();
    Ok(NuveiPaymentsRequest {
        related_transaction_id,
        payment_option: PaymentOption {
            card: Some(Card {
                card_number: Some(card.card_number),
                card_holder_name: Some(card.card_holder_name),
                expiration_month: Some(card.card_exp_month),
                expiration_year: Some(card.card_exp_year),
                three_d,
                cvv: Some(card.card_cvc),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    })
}

impl<F>
    TryFrom<(
        &types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
        String,
    )> for NuveiPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            &types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>,
            String,
        ),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let session_token = data.1;
        if session_token.is_empty() {
            return Err(errors::ConnectorError::MissingRequiredField {
                field_name: "session_token",
            }
            .into());
        }
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.attempt_id.clone();
        let time_stamp = date_time::date_as_yyyymmddhhmmss()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_secret = connector_meta.merchant_secret;
        let request_data = match item.request.payment_method_data.clone() {
            Some(api::PaymentMethodData::Card(card)) => Ok(Self {
                related_transaction_id: item.request.connector_transaction_id.clone(),
                payment_option: PaymentOption {
                    card: Some(Card {
                        card_number: Some(card.card_number),
                        card_holder_name: Some(card.card_holder_name),
                        expiration_month: Some(card.card_exp_month),
                        expiration_year: Some(card.card_exp_year),
                        cvv: Some(card.card_cvc),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment methods".to_string(),
            )),
        }?;
        Ok(Self {
            merchant_id: merchant_id.clone(),
            merchant_site_id: merchant_site_id.clone(),
            client_request_id: client_request_id.clone(),
            amount: item.request.amount.clone().to_string(),
            currency: item.request.currency.clone().to_string(),
            transaction_type: item
                .request
                .capture_method
                .map(TransactionType::from)
                .unwrap_or_default(),
            time_stamp: time_stamp.clone(),
            session_token,
            checksum: encode_payload(vec![
                merchant_id,
                merchant_site_id,
                client_request_id,
                item.request.amount.to_string(),
                item.request.currency.to_string(),
                time_stamp,
                merchant_secret,
            ])?,
            ..request_data
        })
    }
}

impl TryFrom<&types::PaymentsCaptureRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.attempt_id.clone();
        let time_stamp = date_time::date_as_yyyymmddhhmmss()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_secret = connector_meta.merchant_secret;
        Ok(Self {
            merchant_id: merchant_id.clone(),
            merchant_site_id: merchant_site_id.clone(),
            client_request_id: client_request_id.clone(),
            amount: item.request.amount_to_capture.clone().to_string(),
            currency: item.request.currency.clone().to_string(),
            related_transaction_id: Some(item.request.connector_transaction_id.clone()),
            time_stamp: time_stamp.clone(),
            checksum: encode_payload(vec![
                merchant_id,
                merchant_site_id,
                client_request_id,
                item.request.amount_to_capture.to_string(),
                item.request.currency.to_string(),
                item.request.connector_transaction_id.clone(),
                time_stamp,
                merchant_secret,
            ])?,
        })
    }
}

impl TryFrom<&types::RefundExecuteRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundExecuteRouterData) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.attempt_id.clone();
        let time_stamp = date_time::date_as_yyyymmddhhmmss()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_secret = connector_meta.merchant_secret;
        Ok(Self {
            merchant_id: merchant_id.clone(),
            merchant_site_id: merchant_site_id.clone(),
            client_request_id: client_request_id.clone(),
            amount: item.request.amount.clone().to_string(),
            currency: item.request.currency.clone().to_string(),
            related_transaction_id: Some(item.request.connector_transaction_id.clone()),
            time_stamp: time_stamp.clone(),
            checksum: encode_payload(vec![
                merchant_id,
                merchant_site_id,
                client_request_id,
                item.request.amount.to_string(),
                item.request.currency.to_string(),
                item.request.connector_transaction_id.clone(),
                time_stamp,
                merchant_secret,
            ])?,
        })
    }
}

impl TryFrom<&types::PaymentsSyncRouterData> for NuveiPaymentSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let meta: NuveiMeta = utils::to_connector_meta(value.request.connector_meta.clone())?;
        Ok(Self {
            session_token: meta.session_token,
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.attempt_id.clone();
        let time_stamp = date_time::date_as_yyyymmddhhmmss()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_secret = connector_meta.merchant_secret;
        let amount = item.request.get_amount()?.to_string();
        let currency = item.request.get_currency()?.to_string();
        Ok(Self {
            merchant_id: merchant_id.clone(),
            merchant_site_id: merchant_site_id.clone(),
            client_request_id: client_request_id.clone(),
            amount: amount.clone(),
            currency: currency.clone(),
            related_transaction_id: Some(item.request.connector_transaction_id.clone()),
            time_stamp: time_stamp.clone(),
            checksum: encode_payload(vec![
                merchant_id,
                merchant_site_id,
                client_request_id,
                amount,
                currency,
                item.request.connector_transaction_id.clone(),
                time_stamp,
                merchant_secret,
            ])?,
        })
    }
}

// Auth Struct
pub struct NuveiAuthType {
    pub(super) merchant_id: String,
    pub(super) merchant_site_id: String,
    pub(super) merchant_secret: String,
}

impl TryFrom<&types::ConnectorAuthType> for NuveiAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                merchant_id: api_key.to_string(),
                merchant_site_id: key1.to_string(),
                merchant_secret: api_secret.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NuveiPaymentStatus {
    Success,
    Failed,
    Error,
    #[default]
    Processing,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NuveiTransactionStatus {
    Approved,
    Declined,
    Error,
    Redirect,
    #[default]
    Processing,
}

impl From<NuveiTransactionStatus> for enums::AttemptStatus {
    fn from(item: NuveiTransactionStatus) -> Self {
        match item {
            NuveiTransactionStatus::Approved => Self::Charged,
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => Self::Failure,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentsResponse {
    pub order_id: Option<String>,
    pub user_token_id: Option<String>,
    pub payment_option: Option<PaymentOption>,
    pub transaction_status: Option<NuveiTransactionStatus>,
    pub gw_error_code: Option<i64>,
    pub gw_error_reason: Option<String>,
    pub gw_extended_error_code: Option<i64>,
    pub issuer_decline_code: Option<String>,
    pub issuer_decline_reason: Option<String>,
    pub transaction_type: Option<NuveiTransactionType>,
    pub transaction_id: Option<String>,
    pub external_transaction_id: Option<String>,
    pub auth_code: Option<String>,
    pub custom_data: Option<String>,
    pub fraud_details: Option<FraudDetails>,
    pub external_scheme_transaction_id: Option<String>,
    pub session_token: Option<String>,
    pub client_unique_id: Option<String>,
    pub internal_request_id: Option<i64>,
    pub status: NuveiPaymentStatus,
    pub err_code: Option<i64>,
    pub reason: Option<String>,
    pub merchant_id: Option<String>,
    pub merchant_site_id: Option<String>,
    pub version: Option<String>,
    pub client_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NuveiTransactionType {
    Auth,
    Sale,
    Credit,
    Auth3D,
    InitAuth3D,
    Settle,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FraudDetails {
    pub final_decision: String,
}

fn get_payment_status(response: &NuveiPaymentsResponse) -> enums::AttemptStatus {
    match response.transaction_status.clone() {
        Some(status) => match status {
            NuveiTransactionStatus::Approved => match response.transaction_type {
                Some(NuveiTransactionType::Auth) => enums::AttemptStatus::Authorized,
                Some(NuveiTransactionType::Sale) | Some(NuveiTransactionType::Settle) => {
                    enums::AttemptStatus::Charged
                }
                Some(NuveiTransactionType::Void) => enums::AttemptStatus::Voided,
                _ => enums::AttemptStatus::Pending,
            },
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => {
                match response.transaction_type {
                    Some(NuveiTransactionType::Auth) => enums::AttemptStatus::AuthorizationFailed,
                    Some(NuveiTransactionType::Sale) | Some(NuveiTransactionType::Settle) => {
                        enums::AttemptStatus::Failure
                    }
                    Some(NuveiTransactionType::Void) => enums::AttemptStatus::VoidFailed,
                    Some(NuveiTransactionType::Auth3D) => {
                        enums::AttemptStatus::AuthenticationFailed
                    }
                    _ => enums::AttemptStatus::Pending,
                }
            }
            NuveiTransactionStatus::Processing => enums::AttemptStatus::Pending,
            NuveiTransactionStatus::Redirect => enums::AttemptStatus::AuthenticationPending,
        },
        None => match response.status {
            NuveiPaymentStatus::Failed | NuveiPaymentStatus::Error => enums::AttemptStatus::Failure,
            _ => enums::AttemptStatus::Pending,
        },
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, NuveiPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, NuveiPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = match item.data.payment_method {
            storage_models::enums::PaymentMethod::Wallet => item
                .response
                .payment_option
                .as_ref()
                .and_then(|po| po.redirect_url.clone())
                .map(|base_url| services::RedirectForm::from((base_url, services::Method::Get))),
            _ => item
                .response
                .payment_option
                .as_ref()
                .and_then(|o| o.card.clone())
                .and_then(|card| card.three_d)
                .and_then(|three_ds| three_ds.acs_url.zip(three_ds.c_req))
                .map(|(base_url, creq)| services::RedirectForm {
                    endpoint: base_url,
                    method: services::Method::Post,
                    form_fields: std::collections::HashMap::from([("creq".to_string(), creq)]),
                }),
        };

        let response = item.response;
        Ok(Self {
            status: get_payment_status(&response),
            response: match response.status {
                NuveiPaymentStatus::Error => {
                    get_error_response(response.err_code, response.reason, item.http_code)
                }
                _ => match response.transaction_status {
                    Some(NuveiTransactionStatus::Error) => get_error_response(
                        response.gw_error_code,
                        response.gw_error_reason,
                        item.http_code,
                    ),
                    _ => Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: response
                            .transaction_id
                            .map_or(response.order_id, Some) // For paypal there will be no transaction_id, only order_id will be present
                            .map(types::ResponseId::ConnectorTransactionId)
                            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
                        redirection_data,
                        mandate_reference: None,
                        // we don't need to save session token for capture, void flow so ignoring if it is not present
                        connector_metadata: if let Some(token) = response.session_token {
                            Some(
                                serde_json::to_value(NuveiMeta {
                                    session_token: token,
                                })
                                .into_report()
                                .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
                            )
                        } else {
                            None
                        },
                    }),
                },
            },
            ..item.data
        })
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, NuveiACSResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, NuveiACSResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationFailed,
            response: Err(types::ErrorResponse {
                code: consts::NO_ERROR_CODE.to_string(),
                message: "Authentication Failed".to_string(),
                reason: None,
                status_code: item.http_code,
            }),
            ..item.data
        })
    }
}

impl From<NuveiTransactionStatus> for enums::RefundStatus {
    fn from(item: NuveiTransactionStatus) -> Self {
        match item {
            NuveiTransactionStatus::Approved => Self::Success,
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => Self::Failure,
            NuveiTransactionStatus::Processing | NuveiTransactionStatus::Redirect => Self::Pending,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, NuveiPaymentsResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, NuveiPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: get_refund_response(
                item.response.clone(),
                item.http_code,
                item.response
                    .transaction_id
                    .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
            ),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, NuveiPaymentsResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, NuveiPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: get_refund_response(
                item.response.clone(),
                item.http_code,
                item.response
                    .transaction_id
                    .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
            ),
            ..item.data
        })
    }
}

fn get_refund_response(
    response: NuveiPaymentsResponse,
    http_code: u16,
    txn_id: String,
) -> Result<types::RefundsResponseData, types::ErrorResponse> {
    let refund_status = response
        .transaction_status
        .clone()
        .map(enums::RefundStatus::from)
        .unwrap_or(enums::RefundStatus::Failure);
    match response.status {
        NuveiPaymentStatus::Error => {
            get_error_response(response.err_code, response.reason, http_code)
        }
        _ => match response.transaction_status {
            Some(NuveiTransactionStatus::Error) => {
                get_error_response(response.gw_error_code, response.gw_error_reason, http_code)
            }
            _ => Ok(types::RefundsResponseData {
                connector_refund_id: txn_id,
                refund_status,
            }),
        },
    }
}

fn get_error_response<T>(
    error_code: Option<i64>,
    error_msg: Option<String>,
    http_code: u16,
) -> Result<T, types::ErrorResponse> {
    Err(types::ErrorResponse {
        code: error_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
        message: error_msg.unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
        reason: None,
        status_code: http_code,
    })
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NuveiWebhookDetails {
    pub ppp_status: Option<String>,
    #[serde(rename = "ppp_TransactionID")]
    pub ppp_transaction_id: String,
    #[serde(rename = "TransactionId")]
    pub transaction_id: Option<String>,
    pub userid: Option<String>,
    pub merchant_unique_id: Option<String>,
    #[serde(rename = "customData")]
    pub custom_data: Option<String>,
    #[serde(rename = "productId")]
    pub product_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "totalAmount")]
    pub total_amount: String,
    pub currency: String,
    #[serde(rename = "responseTimeStamp")]
    pub response_time_stamp: String,
    #[serde(rename = "Status")]
    pub status: NuveiWebhookStatus,
    #[serde(rename = "transactionType")]
    pub transaction_type: Option<NuveiTransactionType>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NuveiWebhookTransactionId {
    #[serde(rename = "ppp_TransactionID")]
    pub ppp_transaction_id: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NuveiWebhookDataStatus {
    #[serde(rename = "Status")]
    pub status: NuveiWebhookStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NuveiWebhookStatus {
    Approved,
    Declined,
    #[default]
    Pending,
    Update,
}

impl From<NuveiWebhookStatus> for NuveiTransactionStatus {
    fn from(status: NuveiWebhookStatus) -> Self {
        match status {
            NuveiWebhookStatus::Approved => Self::Approved,
            NuveiWebhookStatus::Declined => Self::Declined,
            _ => Self::Processing,
        }
    }
}

impl From<NuveiWebhookDetails> for NuveiPaymentsResponse {
    fn from(item: NuveiWebhookDetails) -> Self {
        Self {
            transaction_status: Some(NuveiTransactionStatus::from(item.status)),
            transaction_id: item.transaction_id,
            transaction_type: item.transaction_type,
            ..Default::default()
        }
    }
}
