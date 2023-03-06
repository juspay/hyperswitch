use common_utils::{
    crypto::{self, GenerateDigest},
    date_time,
};
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{PaymentsCancelRequestData, RouterData},
    consts,
    core::errors,
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

#[derive(Debug, Serialize, Default, Deserialize)]
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

#[derive(Debug, Serialize, Default, Deserialize)]
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
}

#[derive(Debug, Serialize, Default, Deserialize)]
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

#[derive(Debug, Serialize, Default, Deserialize)]
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

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentOption {
    pub card: Card,
    pub user_payment_option_id: Option<String>,
    pub device_details: Option<DeviceDetails>,
    pub billing_address: Option<BillingAddress>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BillingAddress {
    pub email: String,
    pub country: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
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
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeD {
    pub browser_details: Option<BrowserDetails>,
    pub version: Option<String>,
    pub notification_url: Option<String>,
    pub merchant_url: Option<String>,
    pub platform_type: Option<String>,
    pub v2_additional_params: Option<V2AdditionalParams>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserDetails {
    pub accept_header: String,
    pub ip: String,
    pub java_enabled: String,
    pub java_script_enabled: String,
    pub language: String,
    pub color_depth: String,
    pub screen_height: String,
    pub screen_width: String,
    pub time_zone: String,
    pub user_agent: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AdditionalParams {
    pub challenge_window_size: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
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
        let time_stamp = date_time::date_as_yyyymmddhhmmss();
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
    type Error = error_stack::Report<errors::ParsingError>;
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

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NuveiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.attempt_id.clone();
        let time_stamp = date_time::date_as_yyyymmddhhmmss();
        let merchant_secret = connector_meta.merchant_secret;
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(card) => Ok(Self {
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
                payment_option: PaymentOption {
                    card: Card {
                        card_number: Some(card.card_number),
                        card_holder_name: Some(card.card_holder_name),
                        expiration_month: Some(card.card_exp_month),
                        expiration_year: Some(card.card_exp_year),
                        cvv: Some(card.card_cvc),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                time_stamp: time_stamp.clone(),
                session_token: item.get_session_token()?,
                checksum: encode_payload(vec![
                    merchant_id,
                    merchant_site_id,
                    client_request_id,
                    item.request.amount.to_string(),
                    item.request.currency.to_string(),
                    time_stamp,
                    merchant_secret,
                ])?,
                ..Default::default()
            }),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

impl TryFrom<&types::PaymentsCaptureRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.attempt_id.clone();
        let time_stamp = date_time::date_as_yyyymmddhhmmss();
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

impl TryFrom<&types::RefundExecuteRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundExecuteRouterData) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.attempt_id.clone();
        let time_stamp = date_time::date_as_yyyymmddhhmmss();
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
        let meta: NuveiMeta = value.to_connector_meta()?;
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
        let time_stamp = date_time::date_as_yyyymmddhhmmss();
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub enum NuveiTransactionType {
    Auth,
    Sale,
    Credit,
    Settle,
    Void,
}

#[derive(Debug, Serialize, Deserialize)]
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
                    _ => enums::AttemptStatus::Pending,
                }
            }
            NuveiTransactionStatus::Processing => enums::AttemptStatus::Pending,
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
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<F, NuveiPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: get_payment_status(&item.response),
            response: match item.response.status {
                NuveiPaymentStatus::Error => Err(types::ErrorResponse {
                    code: item
                        .response
                        .err_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                    message: item
                        .response
                        .reason
                        .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                    reason: None,
                    status_code: item.http_code,
                }),
                _ => match item.response.transaction_status {
                    Some(NuveiTransactionStatus::Error) => Err(types::ErrorResponse {
                        code: item
                            .response
                            .gw_error_code
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                        message: item
                            .response
                            .gw_error_reason
                            .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                        reason: None,
                        status_code: item.http_code,
                    }),
                    _ => Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(
                            item.response.transaction_id.ok_or(errors::ParsingError)?,
                        ),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: Some(
                            serde_json::to_value(NuveiMeta {
                                session_token: item
                                    .response
                                    .session_token
                                    .ok_or(errors::ParsingError)?,
                            })
                            .into_report()
                            .change_context(errors::ParsingError)?,
                        ),
                    }),
                },
            },
            ..item.data
        })
    }
}

impl From<NuveiTransactionStatus> for enums::RefundStatus {
    fn from(item: NuveiTransactionStatus) -> Self {
        match item {
            NuveiTransactionStatus::Approved => Self::Success,
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => Self::Failure,
            NuveiTransactionStatus::Processing => Self::Pending,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, NuveiPaymentsResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, NuveiPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let response = item.response;
        let http_code = item.http_code;
        let refund_status = response
            .transaction_status
            .clone()
            .map(|a| a.into())
            .unwrap_or(enums::RefundStatus::Failure);
        let refund_response = match response.status {
            NuveiPaymentStatus::Error => Err(types::ErrorResponse {
                code: response
                    .err_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: response
                    .reason
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: None,
                status_code: http_code,
            }),
            _ => match response.transaction_status {
                Some(NuveiTransactionStatus::Error) => Err(types::ErrorResponse {
                    code: response
                        .gw_error_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                    message: response
                        .gw_error_reason
                        .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                    reason: None,
                    status_code: http_code,
                }),
                _ => Ok(types::RefundsResponseData {
                    connector_refund_id: response.transaction_id.ok_or(errors::ParsingError)?,
                    refund_status,
                }),
            },
        };
        Ok(Self {
            response: refund_response,
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, NuveiPaymentsResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, NuveiPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let response = item.response;
        let http_code = item.http_code;
        let refund_status = response
            .transaction_status
            .clone()
            .map(|a| a.into())
            .unwrap_or(enums::RefundStatus::Failure);
        let refund_response = match response.status {
            NuveiPaymentStatus::Error => Err(types::ErrorResponse {
                code: response
                    .err_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: response
                    .reason
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: None,
                status_code: http_code,
            }),
            _ => match response.transaction_status {
                Some(NuveiTransactionStatus::Error) => Err(types::ErrorResponse {
                    code: response
                        .gw_error_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                    message: response
                        .gw_error_reason
                        .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                    reason: None,
                    status_code: http_code,
                }),
                _ => Ok(types::RefundsResponseData {
                    connector_refund_id: response.transaction_id.ok_or(errors::ParsingError)?,
                    refund_status,
                }),
            },
        };
        Ok(Self {
            response: refund_response,
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct NuveiErrorResponse {}
