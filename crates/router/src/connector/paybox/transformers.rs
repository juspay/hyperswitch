use bytes::Bytes;
use common_utils::{date_time::DateFormat, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_connectors::utils::CardData;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils,
    core::errors,
    types::{self, api, domain, storage::enums},
};

pub struct PayboxRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PayboxRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

const AUTH_REQUEST: &str = "00001";
const CAPTURE_REQUEST: &str = "00002";
const AUTH_AND_CAPTURE_REQUEST: &str = "00003";
const SYNC_REQUEST: &str = "00017";
const REFUND_REQUEST: &str = "00014";

const SUCCESS_CODE: &str = "00000";

const VERSION_PAYBOX: &str = "00104";

const PAY_ORIGIN_INTERNET: &str = "024";

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayboxPaymentsRequest {
    #[serde(rename = "DATEQ")]
    pub date: String,

    #[serde(rename = "TYPE")]
    pub transaction_type: String,

    #[serde(rename = "NUMQUESTION")]
    pub paybox_request_number: String,

    #[serde(rename = "MONTANT")]
    pub amount: MinorUnit,

    #[serde(rename = "REFERENCE")]
    pub description_reference: String,

    #[serde(rename = "VERSION")]
    pub version: String,

    #[serde(rename = "DEVISE")]
    pub currency: String,

    #[serde(rename = "PORTEUR")]
    pub card_number: cards::CardNumber,

    #[serde(rename = "DATEVAL")]
    pub expiration_date: Secret<String>,

    #[serde(rename = "CVV")]
    pub cvv: Secret<String>,

    #[serde(rename = "ACTIVITE")]
    pub activity: String,

    #[serde(rename = "SITE")]
    pub site: Secret<String>,

    #[serde(rename = "RANG")]
    pub rank: Secret<String>,

    #[serde(rename = "CLE")]
    pub key: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayboxCaptureRequest {
    #[serde(rename = "DATEQ")]
    pub date: String,

    #[serde(rename = "TYPE")]
    pub transaction_type: String,

    #[serde(rename = "NUMQUESTION")]
    pub paybox_request_number: String,

    #[serde(rename = "MONTANT")]
    pub amount: MinorUnit,

    #[serde(rename = "REFERENCE")]
    pub reference: String,

    #[serde(rename = "VERSION")]
    pub version: String,

    #[serde(rename = "DEVISE")]
    pub currency: String,

    #[serde(rename = "SITE")]
    pub site: Secret<String>,

    #[serde(rename = "RANG")]
    pub rank: Secret<String>,

    #[serde(rename = "CLE")]
    pub key: Secret<String>,

    #[serde(rename = "NUMTRANS")]
    pub transaction_number: String,

    #[serde(rename = "NUMAPPEL")]
    pub call_number: String,
}

impl TryFrom<&PayboxRouterData<&types::PaymentsCaptureRouterData>> for PayboxCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayboxRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth_data: PayboxAuthType =
            PayboxAuthType::try_from(&item.router_data.connector_auth_type)
                .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let currency = diesel_models::enums::Currency::iso_4217(&item.router_data.request.currency)
            .to_string();
        let paybox_meta_data: PayboxMeta =
            utils::to_connector_meta(item.router_data.request.connector_meta.clone())?;
        let format_time = common_utils::date_time::format_date(
            common_utils::date_time::now(),
            DateFormat::YYYYMMDDHHmmss,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            date: format_time.clone(),
            transaction_type: CAPTURE_REQUEST.to_string(),
            paybox_request_number: get_paybox_request_number()?,
            version: VERSION_PAYBOX.to_string(),
            currency,
            site: auth_data.site,
            rank: auth_data.rang,
            key: auth_data.cle,
            transaction_number: paybox_meta_data.connector_request_id,
            call_number: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount,
            reference: item.router_data.request.connector_transaction_id.clone(),
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayboxRsyncRequest {
    #[serde(rename = "DATEQ")]
    pub date: String,

    #[serde(rename = "TYPE")]
    pub transaction_type: String,

    #[serde(rename = "NUMQUESTION")]
    pub paybox_request_number: String,

    #[serde(rename = "VERSION")]
    pub version: String,

    #[serde(rename = "SITE")]
    pub site: Secret<String>,

    #[serde(rename = "RANG")]
    pub rank: Secret<String>,

    #[serde(rename = "CLE")]
    pub key: Secret<String>,

    #[serde(rename = "NUMTRANS")]
    pub transaction_number: String,

    #[serde(rename = "NUMAPPEL")]
    pub call_number: String,
}

impl TryFrom<&types::RefundSyncRouterData> for PayboxRsyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth_data: PayboxAuthType = PayboxAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let format_time = common_utils::date_time::format_date(
            common_utils::date_time::now(),
            DateFormat::YYYYMMDDHHmmss,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let paybox_meta_data: PayboxMeta =
            utils::to_connector_meta(item.request.connector_metadata.clone())?;
        Ok(Self {
            date: format_time.clone(),
            transaction_type: SYNC_REQUEST.to_string(),
            paybox_request_number: get_paybox_request_number()?,
            version: VERSION_PAYBOX.to_string(),
            site: auth_data.site,
            rank: auth_data.rang,
            key: auth_data.cle,
            transaction_number: paybox_meta_data.connector_request_id,
            call_number: item
                .request
                .connector_refund_id
                .clone()
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?,
        })
    }
}
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayboxPSyncRequest {
    #[serde(rename = "DATEQ")]
    pub date: String,

    #[serde(rename = "TYPE")]
    pub transaction_type: String,

    #[serde(rename = "NUMQUESTION")]
    pub paybox_request_number: String,

    #[serde(rename = "VERSION")]
    pub version: String,

    #[serde(rename = "SITE")]
    pub site: Secret<String>,

    #[serde(rename = "RANG")]
    pub rank: Secret<String>,

    #[serde(rename = "CLE")]
    pub key: Secret<String>,

    #[serde(rename = "NUMTRANS")]
    pub transaction_number: String,

    #[serde(rename = "NUMAPPEL")]
    pub call_number: String,
}

impl TryFrom<&types::PaymentsSyncRouterData> for PayboxPSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth_data: PayboxAuthType = PayboxAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let format_time = common_utils::date_time::format_date(
            common_utils::date_time::now(),
            DateFormat::YYYYMMDDHHmmss,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let paybox_meta_data: PayboxMeta =
            utils::to_connector_meta(item.request.connector_meta.clone())?;
        Ok(Self {
            date: format_time.clone(),
            transaction_type: SYNC_REQUEST.to_string(),
            paybox_request_number: get_paybox_request_number()?,
            version: VERSION_PAYBOX.to_string(),
            site: auth_data.site,
            rank: auth_data.rang,
            key: auth_data.cle,
            transaction_number: paybox_meta_data.connector_request_id,
            call_number: item
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayboxMeta {
    pub connector_request_id: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayboxRefundRequest {
    #[serde(rename = "DATEQ")]
    pub date: String,

    #[serde(rename = "TYPE")]
    pub transaction_type: String,

    #[serde(rename = "NUMQUESTION")]
    pub paybox_request_number: String,

    #[serde(rename = "MONTANT")]
    pub amount: MinorUnit,

    #[serde(rename = "VERSION")]
    pub version: String,

    #[serde(rename = "DEVISE")]
    pub currency: String,

    #[serde(rename = "SITE")]
    pub site: Secret<String>,

    #[serde(rename = "RANG")]
    pub rank: Secret<String>,

    #[serde(rename = "CLE")]
    pub key: Secret<String>,

    #[serde(rename = "NUMTRANS")]
    pub transaction_number: String,

    #[serde(rename = "NUMAPPEL")]
    pub call_number: String,
}

impl TryFrom<&PayboxRouterData<&types::PaymentsAuthorizeRouterData>> for PayboxPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayboxRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            domain::PaymentMethodData::Card(req_card) => {
                let auth_data: PayboxAuthType =
                    PayboxAuthType::try_from(&item.router_data.connector_auth_type)
                        .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                let transaction_type =
                    get_transaction_type(item.router_data.request.capture_method)?;
                let currency =
                    diesel_models::enums::Currency::iso_4217(&item.router_data.request.currency)
                        .to_string();
                let expiration_date =
                    req_card.get_card_expiry_month_year_2_digit_with_delimiter("".to_owned())?;
                let format_time = common_utils::date_time::format_date(
                    common_utils::date_time::now(),
                    DateFormat::YYYYMMDDHHmmss,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                Ok(Self {
                    date: format_time.clone(),
                    transaction_type,
                    paybox_request_number: get_paybox_request_number()?,
                    amount: item.amount,
                    description_reference: item
                        .router_data
                        .request
                        .statement_descriptor
                        .clone()
                        .unwrap_or("paybox payment".into()),
                    version: VERSION_PAYBOX.to_string(),
                    currency,
                    card_number: req_card.card_number,
                    expiration_date,
                    cvv: req_card.card_cvc,
                    activity: PAY_ORIGIN_INTERNET.to_string(),
                    site: auth_data.site,
                    rank: auth_data.rang,
                    key: auth_data.cle,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

fn get_transaction_type(capture_method: Option<enums::CaptureMethod>) -> Result<String, Error> {
    match capture_method {
        Some(enums::CaptureMethod::Automatic) => Ok(AUTH_AND_CAPTURE_REQUEST.to_string()),
        Some(enums::CaptureMethod::Manual) => Ok(AUTH_REQUEST.to_string()),
        _ => Err(errors::ConnectorError::CaptureMethodNotSupported)?,
    }
}
fn get_paybox_request_number() -> Result<String, Error> {
    let time_stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .ok_or(errors::ConnectorError::RequestEncodingFailed)?
        .as_millis()
        .to_string();
    // unix time (in milliseconds) has 13 digits.if we consider 8 digits(the number digits to make day deterministic) there is no collision in the paybox_request_number as it will reset the paybox_request_number for each day  and paybox accepting maximum length is 10 so we gonna take 9 (13-9)
    // Ok((time_stamp[4..]).to_string())
    let request_number = time_stamp
        .get(4..)
        .ok_or(errors::ConnectorError::ParsingFailed)?;
    Ok(request_number.to_string())
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayboxAuthType {
    pub(super) site: Secret<String>,
    pub(super) rang: Secret<String>,
    pub(super) cle: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PayboxAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                site: api_key.to_owned(),
                rang: key1.to_owned(),
                cle: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PayboxPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<PayboxPaymentStatus> for enums::AttemptStatus {
    fn from(item: PayboxPaymentStatus) -> Self {
        match item {
            PayboxPaymentStatus::Succeeded => Self::Charged,
            PayboxPaymentStatus::Failed => Self::Failure,
            PayboxPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayboxResponse {
    #[serde(rename = "NUMTRANS")]
    pub transaction_number: String,

    #[serde(rename = "NUMAPPEL")]
    pub call_number: String,

    #[serde(rename = "CODEREPONSE")]
    pub response_code: String,

    #[serde(rename = "COMMENTAIRE")]
    pub response_message: String,
}

pub fn parse_url_encoded_to_struct(
    query_bytes: Bytes,
) -> Result<PayboxResponse, errors::ConnectorError> {
    let query_string = String::from_utf8_lossy(&query_bytes);
    let parsed: std::collections::HashMap<String, String> =
        url::form_urlencoded::parse(query_string.as_bytes())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

    let json_string =
        serde_json::to_string(&parsed).map_err(|_e| errors::ConnectorError::ParsingFailed)?;

    let response: PayboxResponse =
        serde_json::from_str(&json_string).map_err(|_e| errors::ConnectorError::ParsingFailed)?;

    Ok(response)
}

pub fn parse_url_encoded_to_capture_struct(
    query_bytes: Bytes,
) -> Result<PayboxCaptureResponse, errors::ConnectorError> {
    let query_string = String::from_utf8_lossy(&query_bytes);
    let parsed: std::collections::HashMap<String, String> =
        url::form_urlencoded::parse(query_string.as_bytes())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

    let json_string =
        serde_json::to_string(&parsed).map_err(|_e| errors::ConnectorError::ParsingFailed)?;

    let response: PayboxCaptureResponse =
        serde_json::from_str(&json_string).map_err(|_e| errors::ConnectorError::ParsingFailed)?;

    Ok(response)
}

pub fn parse_url_encoded_to_syn_struct(
    query_bytes: Bytes,
) -> Result<PayboxSyncResponse, errors::ConnectorError> {
    let query_string = String::from_utf8_lossy(&query_bytes);

    let parsed: std::collections::HashMap<String, String> =
        url::form_urlencoded::parse(query_string.as_bytes())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

    let json_string =
        serde_json::to_string(&parsed).map_err(|_e| errors::ConnectorError::ParsingFailed)?;

    let response: PayboxSyncResponse =
        serde_json::from_str(&json_string).map_err(|_e| errors::ConnectorError::ParsingFailed)?;

    Ok(response)
}
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PayboxStatus {
    #[serde(rename = "Rembours�")]
    Refunded,

    #[serde(rename = "Annul�")]
    Cancelled,

    #[serde(rename = "Autoris�")]
    #[default]
    Authorised,

    #[serde(rename = "Captur�")]
    Captured,

    #[serde(rename = "Refus�")]
    Rejected,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayboxSyncResponse {
    #[serde(rename = "NUMTRANS")]
    pub transaction_number: String,

    #[serde(rename = "NUMAPPEL")]
    pub call_number: String,

    #[serde(rename = "CODEREPONSE")]
    pub response_code: String,

    #[serde(rename = "COMMENTAIRE")]
    pub response_message: String,

    #[serde(rename = "STATUS")]
    pub status: PayboxStatus,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayboxCaptureResponse {
    #[serde(rename = "NUMTRANS")]
    pub transaction_number: String,

    #[serde(rename = "NUMAPPEL")]
    pub call_number: String,

    #[serde(rename = "CODEREPONSE")]
    pub response_code: String,

    #[serde(rename = "COMMENTAIRE")]
    pub response_message: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PayboxCaptureResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PayboxCaptureResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item.response.clone();
        let status = get_status_of_request(response.response_code.clone());
        match status {
            true => Ok(Self {
                status: enums::AttemptStatus::Pending,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(response.call_number),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: Some(serde_json::json!(PayboxMeta {
                        connector_request_id: response.transaction_number.clone()
                    })),
                    network_txn_id: None,
                    connector_response_reference_id: Some(response.transaction_number),
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                amount_captured: None,
                ..item.data
            }),
            false => Ok(Self {
                response: Err(types::ErrorResponse {
                    code: response.response_code.clone(),
                    message: response.response_message.clone(),
                    reason: Some(response.response_message),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.transaction_number),
                }),
                ..item.data
            }),
        }
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PayboxResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PayboxResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item.response.clone();
        let status = get_status_of_request(response.response_code.clone());
        match status {
            true => Ok(Self {
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(response.call_number),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: Some(serde_json::json!(PayboxMeta {
                        connector_request_id: response.transaction_number.clone()
                    })),
                    network_txn_id: None,
                    connector_response_reference_id: Some(response.transaction_number),
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
            false => Ok(Self {
                response: Err(types::ErrorResponse {
                    code: response.response_code.clone(),
                    message: response.response_message.clone(),
                    reason: Some(response.response_message),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.transaction_number),
                }),
                ..item.data
            }),
        }
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PayboxSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PayboxSyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item.response.clone();
        let status = get_status_of_request(response.response_code.clone());
        let connector_payment_status = item.response.status;
        match status {
            true => Ok(Self {
                status: enums::AttemptStatus::from(connector_payment_status),

                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(response.call_number),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: Some(serde_json::json!(PayboxMeta {
                        connector_request_id: response.transaction_number.clone()
                    })),
                    network_txn_id: None,
                    connector_response_reference_id: Some(response.transaction_number),
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
            false => Ok(Self {
                response: Err(types::ErrorResponse {
                    code: response.response_code.clone(),
                    message: response.response_message.clone(),
                    reason: Some(response.response_message),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.transaction_number),
                }),
                ..item.data
            }),
        }
    }
}

impl From<PayboxStatus> for common_enums::RefundStatus {
    fn from(item: PayboxStatus) -> Self {
        match item {
            PayboxStatus::Refunded => Self::Success,
            PayboxStatus::Cancelled
            | PayboxStatus::Authorised
            | PayboxStatus::Captured
            | PayboxStatus::Rejected => Self::Failure,
        }
    }
}
impl From<PayboxStatus> for enums::AttemptStatus {
    fn from(item: PayboxStatus) -> Self {
        match item {
            PayboxStatus::Refunded => Self::CaptureFailed,
            PayboxStatus::Cancelled => Self::Voided,
            PayboxStatus::Authorised => Self::Authorized,
            PayboxStatus::Captured => Self::Charged,
            PayboxStatus::Rejected => Self::Failure,
        }
    }
}
fn get_status_of_request(item: String) -> bool {
    item == *SUCCESS_CODE
}

impl<F> TryFrom<&PayboxRouterData<&types::RefundsRouterData<F>>> for PayboxRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayboxRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let auth_data: PayboxAuthType =
            PayboxAuthType::try_from(&item.router_data.connector_auth_type)
                .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let currency = diesel_models::enums::Currency::iso_4217(&item.router_data.request.currency)
            .to_string();
        let format_time = common_utils::date_time::format_date(
            common_utils::date_time::now(),
            DateFormat::YYYYMMDDHHmmss,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let paybox_meta_data: PayboxMeta =
            utils::to_connector_meta(item.router_data.request.connector_metadata.clone())?;
        Ok(Self {
            date: format_time.clone(),
            transaction_type: REFUND_REQUEST.to_string(),
            paybox_request_number: get_paybox_request_number()?,
            version: VERSION_PAYBOX.to_string(),
            currency,
            site: auth_data.site,
            rank: auth_data.rang,
            key: auth_data.cle,
            transaction_number: paybox_meta_data.connector_request_id,
            call_number: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount,
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, PayboxSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, PayboxSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_number,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, PayboxResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, PayboxResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_number,
                refund_status: common_enums::RefundStatus::Pending,
            }),
            ..item.data
        })
    }
}
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PayboxErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
