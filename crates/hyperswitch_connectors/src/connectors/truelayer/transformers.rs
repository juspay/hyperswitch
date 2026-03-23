use std::collections::{BTreeMap, HashMap};

use actix_web::http::header::HeaderMap;
#[cfg(feature = "payouts")]
use api_models::payouts::{BankRedirect, PayoutMethodData};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use common_enums::enums;
#[cfg(feature = "payouts")]
use common_utils::pii;
use common_utils::types::MinorUnit;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        refunds::{Execute, RSync},
        VerifyWebhookSource,
    },
    router_request_types::{ResponseId, VerifyWebhookSourceRequestData},
    router_response_types::{
        PaymentsResponseData, RefundsResponseData, VerifyWebhookSourceResponseData,
        VerifyWebhookStatus,
    },
    types::{
        PaymentsAuthorizeRouterData, RefreshTokenRouterData, RefundsRouterData,
        VerifyWebhookSourceRouterData,
    },
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::payouts::PoFulfill, router_response_types::PayoutsResponseData,
    types::PayoutsRouterData,
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use josekit::jws::{JwsHeader, ES512};
use openssl::{
    bn::{BigNum, BigNumContext},
    ec::{EcGroup, EcKey, EcPoint},
    ecdsa::EcdsaSig,
    hash::{hash, MessageDigest},
    nid::Nid,
    pkey::Public,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "payouts")]
use crate::types::PayoutsResponseRouterData;
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, RouterData as OtherRouterData},
};
const PREFIX: &str = "/api";
const GRANT_TYPE: &str = "client_credentials";
pub const ALLOWED_JKUS: &[&str] = &[
    "https://webhooks.truelayer.com/.well-known/jwks",
    "https://webhooks.truelayer-sandbox.com/.well-known/jwks",
];
pub struct TruelayerRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for TruelayerRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TruelayerAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for TruelayerAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                client_id: api_key.to_owned(),
                client_secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruelayerMetadata {
    merchant_account_id: Secret<String>,
    account_holder_name: Secret<String>,
    pub private_key: Secret<String>,
    pub kid: Secret<String>,
}

#[cfg(feature = "payouts")]
impl TryFrom<&Option<pii::SecretSerdeValue>> for TruelayerMetadata {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TruelayerAccessTokenRequestData {
    grant_type: String,
    client_id: Secret<String>,
    client_secret: Secret<String>,
}

impl TryFrom<&RefreshTokenRouterData> for TruelayerAccessTokenRequestData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth = TruelayerAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            grant_type: GRANT_TYPE.to_string(),
            client_id: auth.client_id.clone(),
            client_secret: auth.client_secret.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TruelayerAccessTokenResponseData {
    access_token: Secret<String>,
    expires_in: i64,
    token_type: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, TruelayerAccessTokenResponseData, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, TruelayerAccessTokenResponseData, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TruelayerAccessTokenErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
    pub error_details: Option<ErrorDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorDetails {
    pub reason: Option<String>,
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct TruelayerPaymentsRequest {
    amount: MinorUnit,
    card: TruelayerCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct TruelayerCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&TruelayerRouterData<&PaymentsAuthorizeRouterData>> for TruelayerPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TruelayerRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(_) => Err(errors::ConnectorError::NotImplemented(
                "Card payment method not implemented".to_string(),
            )
            .into()),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TruelayerPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<TruelayerPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: TruelayerPaymentStatus) -> Self {
        match item {
            TruelayerPaymentStatus::Succeeded => Self::Charged,
            TruelayerPaymentStatus::Failed => Self::Failure,
            TruelayerPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TruelayerPaymentsResponse {
    status: TruelayerPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, TruelayerPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, TruelayerPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct TruelayerRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&TruelayerRouterData<&RefundsRouterData<F>>> for TruelayerRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &TruelayerRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
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
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TruelayerErrorResponse {
    #[serde(rename = "type")]
    pub _type: String,
    pub title: String,
    pub status: i32,
    pub trace_id: String,
    pub detail: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TruelayerPayoutRequest {
    merchant_account_id: Secret<String>,
    amount_in_minor: MinorUnit,
    currency: api_models::enums::Currency,
    beneficiary: TruelayerBeneficiary,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TruelayerBeneficiary {
    #[serde(rename = "type")]
    _type: String,
    reference: String,
    account_holder_name: Secret<String>,
    account_identifier: TruelayerAccountIdentifier,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TruelayerAccountIdentifier {
    #[serde(rename = "type")]
    _type: String,
    iban: Secret<String>,
}

#[cfg(feature = "payouts")]
impl TryFrom<&TruelayerRouterData<&PayoutsRouterData<PoFulfill>>> for TruelayerPayoutRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TruelayerRouterData<&PayoutsRouterData<PoFulfill>>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.get_payout_method_data()? {
            PayoutMethodData::BankRedirect(BankRedirect::OpenBankingUk(open_banking_uk_data)) => {
                let metadata = TruelayerMetadata::try_from(&item.router_data.connector_meta_data)?;
                Ok(Self {
                    merchant_account_id: metadata.merchant_account_id,
                    amount_in_minor: item.amount,
                    currency: item.router_data.request.destination_currency,
                    beneficiary: TruelayerBeneficiary {
                        _type: "external_account".to_string(),
                        reference: normalize_payment_id(
                            item.router_data.request.payout_id.get_string_repr(),
                        ),
                        account_holder_name: open_banking_uk_data.account_holder_name,
                        account_identifier: TruelayerAccountIdentifier {
                            _type: "iban".to_string(),
                            iban: open_banking_uk_data.iban,
                        },
                    },
                })
            }
            _ => Err(errors::ConnectorError::NotSupported {
                message: "Payout Method Not Supported".to_string(),
                connector: "Truelayer",
            })?,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TruelayerPayoutResponse {
    id: String,
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, TruelayerPayoutResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, TruelayerPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(common_enums::PayoutStatus::Initiated),
                connector_payout_id: Some(item.response.id.clone()),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
                payout_connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TruelayerPayoutSyncType {
    Sync(TruelayerPayoutSyncResponse),
    Webhook(TruelayerPayoutsWebhookBody),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TruelayerPayoutSyncResponse {
    id: String,
    merchant_account_id: Secret<String>,
    amount_in_minor: MinorUnit,
    currency: api_models::enums::Currency,
    beneficiary: TruelayerBeneficiary,
    scheme_id: Option<String>,
    status: TruelayerPayoutStatus,
    failed_at: Option<String>,
    failure_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum TruelayerPayoutStatus {
    Pending,
    AuthorizationRequired,
    Failed,
    Authorizing,
    Authorized,
    Executed,
}

#[cfg(feature = "payouts")]
impl From<TruelayerPayoutStatus> for enums::PayoutStatus {
    fn from(item: TruelayerPayoutStatus) -> Self {
        match item {
            TruelayerPayoutStatus::Pending
            | TruelayerPayoutStatus::AuthorizationRequired
            | TruelayerPayoutStatus::Authorizing => Self::Pending,
            TruelayerPayoutStatus::Authorized | TruelayerPayoutStatus::Executed => Self::Success,
            TruelayerPayoutStatus::Failed => Self::Failed,
        }
    }
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, TruelayerPayoutSyncType>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, TruelayerPayoutSyncType>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            TruelayerPayoutSyncType::Sync(sync_response) => {
                let status = enums::PayoutStatus::from(sync_response.status);
                if status == enums::PayoutStatus::Failed {
                    let failure_reason = sync_response.failure_reason.ok_or(
                        errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                            "Expected failure reason for failed payout".to_string(),
                        )),
                    )?;

                    Ok(Self {
                        response: Err(ErrorResponse {
                            code: failure_reason.clone(),
                            message: failure_reason.clone(),
                            reason: Some(failure_reason.clone()),
                            attempt_status: None,
                            connector_transaction_id: Some(sync_response.id.clone()),
                            connector_response_reference_id: None,
                            status_code: item.http_code,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        response: Ok(PayoutsResponseData {
                            status: Some(status),
                            connector_payout_id: Some(sync_response.id.to_string()),
                            payout_eligible: None,
                            should_add_next_step_to_process_tracker: false,
                            error_code: None,
                            error_message: None,
                            payout_connector_metadata: None,
                        }),
                        ..item.data
                    })
                }
            }
            TruelayerPayoutSyncType::Webhook(webhook_response) => {
                let status = match webhook_response._type {
                    TruelayerPayoutsWebhookEvent::PayoutExecuted => enums::PayoutStatus::Success,
                    TruelayerPayoutsWebhookEvent::PayoutFailed => enums::PayoutStatus::Failed,
                };

                if status == enums::PayoutStatus::Failed {
                    let failure_reason = webhook_response.failure_reason.ok_or(
                        errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                            "Expected failure reason for failed payout".to_string(),
                        )),
                    )?;

                    Ok(Self {
                        response: Err(ErrorResponse {
                            code: failure_reason.clone(),
                            message: failure_reason.clone(),
                            reason: Some(failure_reason.clone()),
                            attempt_status: None,
                            connector_transaction_id: Some(webhook_response.payout_id.clone()),
                            connector_response_reference_id: Some(webhook_response.event_id),
                            status_code: item.http_code,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        }),
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        response: Ok(PayoutsResponseData {
                            status: Some(status),
                            connector_payout_id: Some(webhook_response.payout_id.to_string()),
                            payout_eligible: None,
                            should_add_next_step_to_process_tracker: false,
                            error_code: None,
                            error_message: None,
                            payout_connector_metadata: None,
                        }),
                        ..item.data
                    })
                }
            }
        }
    }
}

fn normalize_path(path: &str) -> &str {
    path.trim_end_matches('/')
}

pub fn build_payload(
    method: String,
    path: &str,
    headers: &BTreeMap<String, String>,
    body: Option<&str>,
) -> String {
    let mut payload = format!("{} {}\n", method.to_uppercase(), normalize_path(path));

    for (k, v) in headers {
        payload.push_str(&format!("{}: {}\n", k, v));
    }

    if let Some(body_str) = body {
        payload.push_str(body_str);
    }

    payload
}

pub fn generate_tl_signature(
    method: String,
    path: &str,
    headers: &BTreeMap<String, String>,
    body: Option<&str>,
    private_key: String,
    kid: &str,
) -> common_utils::errors::CustomResult<String, errors::ConnectorError> {
    let payload = build_payload(method, path, headers, body);
    let pem = utils::base64_decode(private_key)?;

    let signer = ES512.signer_from_pem(&pem).change_context(
        errors::ConnectorError::RequestEncodingFailedWithReason(
            "Failed to generate Tl-Signature".to_string(),
        ),
    )?;

    let tl_headers = headers.keys().cloned().collect::<Vec<_>>().join(",");

    let mut header = JwsHeader::new();
    header.set_algorithm("ES512");
    header.set_key_id(kid);
    header
        .set_claim("tl_version", Some("2".into()))
        .change_context(errors::ConnectorError::RequestEncodingFailedWithReason(
            "Failed to generate Tl-Signature".to_string(),
        ))?;
    header
        .set_claim("tl_headers", Some(tl_headers.into()))
        .change_context(errors::ConnectorError::RequestEncodingFailedWithReason(
            "Failed to generate Tl-Signature".to_string(),
        ))?;

    let jws = josekit::jws::serialize_compact(payload.as_bytes(), &header, &signer)
        .change_context(errors::ConnectorError::RequestEncodingFailedWithReason(
            "Failed to generate Tl-Signature".to_string(),
        ))?;

    let parts: Vec<&str> = jws.split('.').collect();

    match (parts.first(), parts.get(2)) {
        (Some(first), Some(third)) => Ok(format!("{}..{}", first, third)),
        _ => Err(errors::ConnectorError::RequestEncodingFailedWithReason(
            "Failed to generate Tl-Signature".to_string(),
        )
        .into()),
    }
}

pub fn normalize_payment_id(payment_id: &str) -> String {
    payment_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TruelayerPayoutsWebhookEvent {
    PayoutExecuted,
    PayoutFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TruelayerWebhookEventType {
    PaymentAuthorized,
    PaymentExecuted,
    PaymentCreditable,
    PaymentSettled,
    PaymentFailed,
    PaymentSettlementStalled,
    PaymentDisputed,
    PaymentReversed,
    PaymentFundsReceived,
    RefundExecuted,
    RefundFailed,
    PayoutExecuted,
    PayoutFailed,
}

impl TruelayerWebhookEventType {
    pub fn is_payout_webhook_event(self) -> bool {
        matches!(self, Self::PayoutExecuted | Self::PayoutFailed)
    }

    pub fn is_payment_webhook_event(self) -> bool {
        matches!(
            self,
            Self::PaymentAuthorized
                | Self::PaymentExecuted
                | Self::PaymentSettled
                | Self::PaymentFailed
                | Self::PaymentCreditable
                | Self::PaymentSettlementStalled
                | Self::PaymentReversed
                | Self::PaymentFundsReceived
        )
    }

    pub fn is_refund_webhook_event(self) -> bool {
        matches!(self, Self::RefundExecuted | Self::RefundFailed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TruelayerWebhookEventBody {
    #[serde(rename = "type")]
    pub _type: TruelayerWebhookEventType,
    pub payout_id: Option<String>,
    pub refund_id: Option<String>,
    pub payment_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TruelayerPayoutsWebhookBody {
    #[serde(rename = "type")]
    pub _type: TruelayerPayoutsWebhookEvent,
    pub event_version: i32,
    pub event_id: String,
    pub payout_id: String,
    pub executed_at: Option<String>,
    pub failed_at: Option<String>,
    pub failure_reason: Option<String>,
    pub scheme_id: Option<String>,
}

pub fn get_payout_webhook_event(
    event: TruelayerPayoutsWebhookEvent,
) -> api_models::webhooks::IncomingWebhookEvent {
    match event {
        TruelayerPayoutsWebhookEvent::PayoutExecuted => {
            api_models::webhooks::IncomingWebhookEvent::PayoutSuccess
        }
        TruelayerPayoutsWebhookEvent::PayoutFailed => {
            api_models::webhooks::IncomingWebhookEvent::PayoutFailure
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
struct Jwk {
    kid: String,
    kty: String,
    x: Option<String>,
    y: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JwsHeaderWebhooks {
    pub jku: Option<String>,
    kid: String,
    tl_headers: Option<String>,
}

fn pad_to(bytes: Vec<u8>, target: usize) -> Result<Vec<u8>, errors::ConnectorError> {
    match bytes.len().cmp(&target) {
        std::cmp::Ordering::Equal => Ok(bytes),
        std::cmp::Ordering::Less => {
            let mut padded = vec![0u8; target - bytes.len()];
            padded.extend(bytes);
            Ok(padded)
        }
        std::cmp::Ordering::Greater => Err(errors::ConnectorError::WebhookSourceVerificationFailed),
    }
}

fn headermap_to_hashmap(headers: &HeaderMap) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for (key, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            map.insert(key.to_string(), value_str.to_string());
        }
    }

    map
}

fn verify_signature(
    body: &[u8],
    jws_header: JwsHeaderWebhooks,
    header_b64: &str,
    signature_b64: &str,
    headers: &HashMap<String, String>,
    ec_key: &EcKey<Public>,
    webhook_uri: &str,
) -> Result<bool, error_stack::Report<errors::ConnectorError>> {
    let tl_headers_str = jws_header.tl_headers.unwrap_or_default();
    let mut payload: Vec<u8> = format!("{} {}\n", "POST".to_uppercase(), webhook_uri).into_bytes();

    if !tl_headers_str.is_empty() {
        let lower_headers: HashMap<String, &String> =
            headers.iter().map(|(k, v)| (k.to_lowercase(), v)).collect();
        for header_name in tl_headers_str.split(',') {
            let name = header_name.trim();
            let value = lower_headers
                .get(&name.to_lowercase())
                .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;
            payload.extend_from_slice(format!("{}: {}\n", name, value).as_bytes());
        }
    }
    payload.extend_from_slice(body);

    // signing_input = base64url(header) + "." + base64url(payload)
    let signing_input = format!("{}.{}", header_b64, URL_SAFE_NO_PAD.encode(&payload));

    // Convert P1363 signature (r || s, 66 bytes each) to DER
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
    if sig_bytes.len() != 132 {
        return Err(report!(
            errors::ConnectorError::WebhookSourceVerificationFailed
        ));
    }

    let r = BigNum::from_slice(
        sig_bytes
            .get(0..66)
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?,
    )
    .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
    let s = BigNum::from_slice(
        sig_bytes
            .get(66..)
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?,
    )
    .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
    let der_sig = EcdsaSig::from_private_components(r, s)
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?
        .to_der()
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

    // SHA-512 digest + ECDSA verify
    let digest = hash(MessageDigest::sha512(), signing_input.as_bytes())
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

    let ecdsa_sig = EcdsaSig::from_der(&der_sig)
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

    let valid = ecdsa_sig
        .verify(&digest, ec_key)
        .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

    Ok(valid)
}

impl
    TryFrom<
        ResponseRouterData<
            VerifyWebhookSource,
            Jwks,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
    > for VerifyWebhookSourceRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            VerifyWebhookSource,
            Jwks,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let body = item.data.request.webhook_body.as_ref();
        let headers = headermap_to_hashmap(&item.data.request.webhook_headers);

        let tl_signature_header = item
            .data
            .request
            .webhook_headers
            .get("Tl-Signature")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        let tl_signature = tl_signature_header
            .to_str()
            .map_err(|_| errors::ConnectorError::WebhookSignatureNotFound)?;
        let parts: Vec<&str> = tl_signature.splitn(3, '.').collect();

        let header_b64 = parts
            .first()
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let signature_b64 = parts
            .get(2)
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let header_json = URL_SAFE_NO_PAD
            .decode(header_b64)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let jws_header: JwsHeaderWebhooks = serde_json::from_slice(&header_json)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let jwk = item
            .response
            .keys
            .into_iter()
            .find(|k| k.kid == jws_header.kid && k.kty == "EC")
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let x_raw = URL_SAFE_NO_PAD
            .decode(
                jwk.x
                    .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let y_raw = URL_SAFE_NO_PAD
            .decode(
                jwk.y
                    .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let mut sec1 = vec![0x04u8];
        sec1.extend(pad_to(x_raw, 66)?);
        sec1.extend(pad_to(y_raw, 66)?);

        let group = EcGroup::from_curve_name(Nid::SECP521R1)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let mut ctx = BigNumContext::new()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let point = EcPoint::from_bytes(&group, &sec1, &mut ctx)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let ec_key = EcKey::from_public_key(&group, &point)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        ec_key
            .check_key()
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let webhook_uri = item.data.request.webhook_uri.to_string();

        let valid = verify_signature(
            body,
            jws_header.clone(),
            header_b64,
            signature_b64,
            &headers,
            &ec_key,
            &(PREFIX.to_owned() + &webhook_uri),
        )? || verify_signature(
            body,
            jws_header.clone(),
            header_b64,
            signature_b64,
            &headers,
            &ec_key,
            &webhook_uri,
        )?;

        Ok(Self {
            response: Ok(VerifyWebhookSourceResponseData {
                verify_webhook_status: if valid {
                    VerifyWebhookStatus::SourceVerified
                } else {
                    VerifyWebhookStatus::SourceNotVerified
                },
            }),
            ..item.data
        })
    }
}
