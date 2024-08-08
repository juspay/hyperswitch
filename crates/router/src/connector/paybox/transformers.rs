use bytes::Bytes;
use common_utils::{date_time::DateFormat, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_connectors::utils::CardData;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
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

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayboxPaymentsRequest {
    #[serde(rename = "DATEQ")]
    pub date: String,

    #[serde(rename = "TYPE")]
    pub transaction_type: String,

    #[serde(rename = "NUMQUESTION")]
    pub question_number: String,

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
    pub activity: String, // always 024

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
    pub question_number: String,

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
        let current_time = common_utils::date_time::now();
        let format_time =
            common_utils::date_time::format_date(current_time, DateFormat::YYYYMMDDHHmmss).unwrap();
        Ok(Self {
            date: format_time.clone(),
            transaction_type: "00002".into(),
            question_number: get_question_number(),
            version: get_version(),
            currency,
            site: auth_data.site,
            rank: auth_data.rang,
            key: auth_data.cle,
            transaction_number: "k".into(),
            call_number: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount,
            reference: item.router_data.request.connector_transaction_id.clone(),
        })
    }
}
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayboxVoidRequest {
    #[serde(rename = "DATEQ")]
    pub date: String,

    #[serde(rename = "TYPE")]
    pub transaction_type: String,

    #[serde(rename = "NUMQUESTION")]
    pub question_number: String,

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

impl TryFrom<&PayboxRouterData<&types::PaymentsCancelRouterData>> for PayboxVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayboxRouterData<&types::PaymentsCancelRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth_data: PayboxAuthType =
            PayboxAuthType::try_from(&item.router_data.connector_auth_type)
                .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let currency =
            diesel_models::enums::Currency::iso_4217(&item.router_data.request.currency.unwrap())
                .to_string();
        let current_time = common_utils::date_time::now();
        let format_time =
            common_utils::date_time::format_date(current_time, DateFormat::YYYYMMDDHHmmss).unwrap();
        Ok(Self {
            date: format_time.clone(),
            transaction_type: "00005".into(),
            question_number: get_question_number(),
            version: get_version(),
            currency,
            site: auth_data.site,
            rank: auth_data.rang,
            key: auth_data.cle,
            transaction_number: "k".into(),
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
    pub question_number: String,

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
        let current_time = common_utils::date_time::now();
        let format_time =
            common_utils::date_time::format_date(current_time, DateFormat::YYYYMMDDHHmmss).unwrap();
        Ok(Self {
            date: format_time.clone(),
            transaction_type: "00017".into(),
            question_number: get_question_number(),
            version: get_version(),
            site: auth_data.site,
            rank: auth_data.rang,
            key: auth_data.cle,
            transaction_number: "k".into(),
            call_number: item.payment_id.clone(),
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
    pub question_number: String,

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
        let current_time = common_utils::date_time::now();
        let format_time =
            common_utils::date_time::format_date(current_time, DateFormat::YYYYMMDDHHmmss).unwrap();
        Ok(Self {
            date: format_time.clone(),
            transaction_type: "00017".into(),
            question_number: get_question_number(),
            version: get_version(),
            site: auth_data.site,
            rank: auth_data.rang,
            key: auth_data.cle,
            transaction_number: "k".into(),
            call_number: item.payment_id.clone(),
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct PayboxRefundRequest {
    #[serde(rename = "DATEQ")]
    pub date: String,

    #[serde(rename = "TYPE")]
    pub transaction_type: String,

    #[serde(rename = "NUMQUESTION")]
    pub question_number: String,

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
                let current_time = common_utils::date_time::now();
                let format_time =
                    common_utils::date_time::format_date(current_time, DateFormat::YYYYMMDDHHmmss)
                        .unwrap();

                Ok(Self {
                    date: format_time.clone(),
                    transaction_type,
                    question_number: get_question_number(),
                    amount: item.amount,
                    description_reference: item
                        .router_data
                        .request
                        .statement_descriptor
                        .clone()
                        .unwrap_or("paybox payment".into()),
                    version: get_version(),
                    currency,
                    card_number: req_card.card_number,
                    expiration_date,
                    cvv: req_card.card_cvc,
                    activity: get_activity(),
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
        Some(enums::CaptureMethod::Automatic) => Ok("00003".into()),
        Some(enums::CaptureMethod::Manual) => Ok("00001".into()),
        _ => Err(errors::ConnectorError::CaptureMethodNotSupported)?,
    }
}
fn get_question_number() -> String {
    let time_stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .ok_or(errors::ConnectorError::RequestEncodingFailed)
        .unwrap()
        .as_millis()
        .to_string();
    // unix time in ms has 13 digits .if we consider 8 digits in a day there is no collision in the digits and paybox accepting maximum length is 10 so we gonna take 9 (13-9)
    (&time_stamp[4..]).to_string()
}
fn get_version() -> String {
    "00104".into()
}

fn get_activity() -> String {
    "024".into()
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

    #[serde(rename = "STATUS")]
    pub status: Option<PayboxStatus>,
}

pub fn parse_url_encoded_to_struct(
    query_bytes: Bytes,
) -> Result<PayboxResponse, errors::ConnectorError> {
    let query_string = String::from_utf8_lossy(&query_bytes);
    let parsed: std::collections::HashMap<String, String> =
        url::form_urlencoded::parse(query_string.as_bytes())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

    let json_string = serde_json::to_string(&parsed).unwrap();

    let response: PayboxResponse = serde_json::from_str(&json_string).unwrap();
    Ok(response)
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PayboxStatus {
    #[serde(rename = "Remboursé")]
    Refunded,

    #[serde(rename = "Annulé")]
    Cancelled,

    #[serde(rename = "Autorisé")]
    #[default]
    Authorised,

    #[serde(rename = "Capturé")]
    Captured,

    #[serde(rename = "Refusé")]
    Rejected,
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
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(response.transaction_number),
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
            false => Ok(Self {
                // status: enums::AttemptStatus::from(response.status),
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
impl From<PayboxStatus> for enums::AttemptStatus {
    fn from(item: PayboxStatus) -> Self {
        match item {
            PayboxStatus::Refunded => enums::AttemptStatus::CaptureFailed,
            PayboxStatus::Cancelled => enums::AttemptStatus::Voided,
            PayboxStatus::Authorised => enums::AttemptStatus::Authorized,
            PayboxStatus::Captured => enums::AttemptStatus::Charged,
            PayboxStatus::Rejected => enums::AttemptStatus::Failure,
        }
    }
}
fn get_status_of_request(item: String) -> bool {
    item == "00000".to_string()
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
        let current_time = common_utils::date_time::now();
        let format_time =
            common_utils::date_time::format_date(current_time, DateFormat::YYYYMMDDHHmmss).unwrap();
        Ok(Self {
            date: format_time.clone(),
            transaction_type: "00014".into(),
            question_number: get_question_number(),
            version: get_version(),
            currency,
            site: auth_data.site,
            rank: auth_data.rang,
            key: auth_data.cle,
            transaction_number: "k".into(),
            call_number: item.router_data.request.refund_id.clone(),
            amount: item.amount,
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, PayboxResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, PayboxResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.call_number,
                refund_status: enums::RefundStatus::Pending,
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
                connector_refund_id: item.response.call_number,
                refund_status: enums::RefundStatus::Pending,
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
