use bytes::Bytes;
use common_utils::{
    date_time::DateFormat, errors::CustomResult, ext_traits::ValueExt, types::MinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_connectors::utils::{AddressDetailsData, CardData};
use hyperswitch_domain_models::{
    router_data::ConnectorAuthType, router_response_types::RedirectForm,
};
use hyperswitch_interfaces::consts;
use masking::{PeekInterface, Secret};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData, RouterData,
    },
    core::errors,
    types::{self, api, domain, storage::enums},
};

pub struct PayboxRouterData<T> {
    pub amount: MinorUnit,
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
const THREE_DS_FAIL_CODE: &str = "00000000";

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PayboxPaymentsRequest {
    Card(PaymentsRequest),
    CardThreeDs(ThreeDSPaymentsRequest),
}

#[derive(Debug, Serialize)]
pub struct PaymentsRequest {
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

    #[serde(rename = "ID3D")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub three_ds_data: Option<Secret<String>>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ThreeDSPaymentsRequest {
    id_merchant: Secret<String>,
    id_session: String,
    amount: MinorUnit,
    currency: String,
    #[serde(rename = "CCNumber")]
    cc_number: cards::CardNumber,
    #[serde(rename = "CCExpDate")]
    cc_exp_date: Secret<String>,
    #[serde(rename = "CVVCode")]
    cvv_code: Secret<String>,
    #[serde(rename = "URLRetour")]
    url_retour: String,
    #[serde(rename = "URLHttpDirect")]
    url_http_direct: String,
    email_porteur: common_utils::pii::Email,
    first_name: Secret<String>,
    last_name: Secret<String>,
    address1: Secret<String>,
    zip_code: Secret<String>,
    city: String,
    country_code: api_models::enums::CountryAlpha2,
    total_quantity: i32,
}
#[derive(Debug, Serialize, Eq, PartialEq)]
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
    pub paybox_order_id: String,
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
            DateFormat::DDMMYYYYHHmmss,
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
            paybox_order_id: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount,
            reference: item.router_data.connector_request_reference_id.to_string(),
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
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
    pub paybox_order_id: String,
}

impl TryFrom<&types::RefundSyncRouterData> for PayboxRsyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth_data: PayboxAuthType = PayboxAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let format_time = common_utils::date_time::format_date(
            common_utils::date_time::now(),
            DateFormat::DDMMYYYYHHmmss,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            date: format_time.clone(),
            transaction_type: SYNC_REQUEST.to_string(),
            paybox_request_number: get_paybox_request_number()?,
            version: VERSION_PAYBOX.to_string(),
            site: auth_data.site,
            rank: auth_data.rang,
            key: auth_data.cle,
            transaction_number: item
                .request
                .connector_refund_id
                .clone()
                .ok_or(errors::ConnectorError::RequestEncodingFailed)?,
            paybox_order_id: item.request.connector_transaction_id.clone(),
        })
    }
}
#[derive(Debug, Serialize, Eq, PartialEq)]
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
    pub paybox_order_id: String,
}

impl TryFrom<&types::PaymentsSyncRouterData> for PayboxPSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth_data: PayboxAuthType = PayboxAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let format_time = common_utils::date_time::format_date(
            common_utils::date_time::now(),
            DateFormat::DDMMYYYYHHmmss,
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
            paybox_order_id: item
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

#[derive(Debug, Serialize, Eq, PartialEq)]
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
    pub paybox_order_id: String,
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
                    DateFormat::DDMMYYYYHHmmss,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                if item.router_data.is_three_ds() {
                    let address = item.router_data.get_billing_address()?;
                    Ok(Self::CardThreeDs(ThreeDSPaymentsRequest {
                        id_merchant: auth_data.merchant_id,
                        id_session: item.router_data.connector_request_reference_id.clone(),
                        amount: item.amount,
                        currency,
                        cc_number: req_card.card_number,
                        cc_exp_date: expiration_date,
                        cvv_code: req_card.card_cvc,
                        url_retour: item.router_data.request.get_complete_authorize_url()?,
                        url_http_direct: item.router_data.request.get_complete_authorize_url()?,
                        email_porteur: item.router_data.request.get_email()?,
                        first_name: address.get_first_name()?.clone(),
                        last_name: address.get_last_name()?.clone(),
                        address1: address.get_line1()?.clone(),
                        zip_code: address.get_zip()?.clone(),
                        city: address.get_city()?.clone(),
                        country_code: *address.get_country()?,
                        total_quantity: 1,
                    }))
                } else {
                    Ok(Self::Card(PaymentsRequest {
                        date: format_time.clone(),
                        transaction_type,
                        paybox_request_number: get_paybox_request_number()?,
                        amount: item.amount,
                        description_reference: item
                            .router_data
                            .connector_request_reference_id
                            .clone(),
                        version: VERSION_PAYBOX.to_string(),
                        currency,
                        card_number: req_card.card_number,
                        expiration_date,
                        cvv: req_card.card_cvc,
                        activity: PAY_ORIGIN_INTERNET.to_string(),
                        site: auth_data.site,
                        rank: auth_data.rang,
                        key: auth_data.cle,
                        three_ds_data: None,
                    }))
                }
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

fn get_transaction_type(capture_method: Option<enums::CaptureMethod>) -> Result<String, Error> {
    match capture_method {
        Some(enums::CaptureMethod::Automatic) | None => Ok(AUTH_AND_CAPTURE_REQUEST.to_string()),
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
    let request_number = time_stamp
        .get(4..)
        .ok_or(errors::ConnectorError::ParsingFailed)?;
    Ok(request_number.to_string())
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct PayboxAuthType {
    pub(super) site: Secret<String>,
    pub(super) rang: Secret<String>,
    pub(super) cle: Secret<String>,
    pub(super) merchant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PayboxAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::MultiAuthKey {
            api_key,
            key1,
            api_secret,
            key2,
        } = auth_type
        {
            Ok(Self {
                site: api_key.to_owned(),
                rang: key1.to_owned(),
                cle: api_secret.to_owned(),
                merchant_id: key2.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PayboxResponse {
    NonThreeDs(TransactionResponse),
    ThreeDs(Secret<String>),
    Error(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    #[serde(rename = "NUMTRANS")]
    pub transaction_number: String,

    #[serde(rename = "NUMAPPEL")]
    pub paybox_order_id: String,

    #[serde(rename = "CODEREPONSE")]
    pub response_code: String,

    #[serde(rename = "COMMENTAIRE")]
    pub response_message: String,
}

pub fn parse_url_encoded_to_struct<T: DeserializeOwned>(
    query_bytes: Bytes,
) -> CustomResult<T, errors::ConnectorError> {
    let (cow, _, _) = encoding_rs::ISO_8859_10.decode(&query_bytes);
    serde_qs::from_str::<T>(cow.as_ref()).change_context(errors::ConnectorError::ParsingFailed)
}

pub fn parse_paybox_response(
    query_bytes: Bytes,
    is_three_ds: bool,
) -> CustomResult<PayboxResponse, errors::ConnectorError> {
    let (cow, _, _) = encoding_rs::ISO_8859_10.decode(&query_bytes);
    let response_str = cow.as_ref();

    if response_str.starts_with("<html>") && is_three_ds {
        let response = response_str.to_string();
        return Ok(if response.contains("Erreur") {
            PayboxResponse::Error(response)
        } else {
            PayboxResponse::ThreeDs(response.into())
        });
    }

    serde_qs::from_str::<TransactionResponse>(response_str)
        .map(PayboxResponse::NonThreeDs)
        .change_context(errors::ConnectorError::ParsingFailed)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PayboxStatus {
    #[serde(rename = "Remboursé")]
    Refunded,

    #[serde(rename = "Annulé")]
    Cancelled,

    #[serde(rename = "Autorisé")]
    Authorised,

    #[serde(rename = "Capturé")]
    Captured,

    #[serde(rename = "Refusé")]
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayboxSyncResponse {
    #[serde(rename = "NUMTRANS")]
    pub transaction_number: String,

    #[serde(rename = "NUMAPPEL")]
    pub paybox_order_id: String,

    #[serde(rename = "CODEREPONSE")]
    pub response_code: String,

    #[serde(rename = "COMMENTAIRE")]
    pub response_message: String,

    #[serde(rename = "STATUS")]
    pub status: PayboxStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayboxCaptureResponse {
    #[serde(rename = "NUMTRANS")]
    pub transaction_number: String,

    #[serde(rename = "NUMAPPEL")]
    pub paybox_order_id: String,

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
                status: enums::AttemptStatus::Charged,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        response.paybox_order_id,
                    ),
                    redirection_data: None,
                    mandate_reference: Box::new(None),
                    connector_metadata: Some(serde_json::json!(PayboxMeta {
                        connector_request_id: response.transaction_number.clone()
                    })),
                    network_txn_id: None,
                    connector_response_reference_id: None,
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

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            PayboxResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            PayboxResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.clone() {
            PayboxResponse::NonThreeDs(response) => {
                let status = get_status_of_request(response.response_code.clone());
                match status {
                    true => Ok(Self {
                        status: match item.data.request.is_auto_capture()? {
                            true => enums::AttemptStatus::Charged,
                            false => enums::AttemptStatus::Authorized,
                        },
                        response: Ok(types::PaymentsResponseData::TransactionResponse {
                            resource_id: types::ResponseId::ConnectorTransactionId(
                                response.paybox_order_id,
                            ),
                            redirection_data: None,
                            mandate_reference: Box::new(None),
                            connector_metadata: Some(serde_json::json!(PayboxMeta {
                                connector_request_id: response.transaction_number.clone()
                            })),
                            network_txn_id: None,
                            connector_response_reference_id: None,
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
                            connector_transaction_id: Some(response.transaction_number),
                        }),
                        ..item.data
                    }),
                }
            }
            PayboxResponse::ThreeDs(data) => Ok(Self {
                status: enums::AttemptStatus::AuthenticationPending,
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::NoResponseId,
                    redirection_data: Some(RedirectForm::Html {
                        html_data: data.peek().to_string(),
                    }),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                }),
                ..item.data
            }),
            PayboxResponse::Error(_) => Ok(Self {
                response: Err(types::ErrorResponse {
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: consts::NO_ERROR_MESSAGE.to_string(),
                    reason: Some(consts::NO_ERROR_MESSAGE.to_string()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
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
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        response.paybox_order_id,
                    ),
                    redirection_data: None,
                    mandate_reference: Box::new(None),
                    connector_metadata: Some(serde_json::json!(PayboxMeta {
                        connector_request_id: response.transaction_number.clone()
                    })),
                    network_txn_id: None,
                    connector_response_reference_id: None,
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
            PayboxStatus::Cancelled => Self::Voided,
            PayboxStatus::Authorised => Self::Authorized,
            PayboxStatus::Captured | PayboxStatus::Refunded => Self::Charged,
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
            DateFormat::DDMMYYYYHHmmss,
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
            paybox_order_id: item.router_data.request.connector_transaction_id.clone(),
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
        let status = get_status_of_request(item.response.response_code.clone());
        match status {
            true => Ok(Self {
                response: Ok(types::RefundsResponseData {
                    connector_refund_id: item.response.transaction_number,
                    refund_status: enums::RefundStatus::from(item.response.status),
                }),
                ..item.data
            }),
            false => Ok(Self {
                response: Err(types::ErrorResponse {
                    code: item.response.response_code.clone(),
                    message: item.response.response_message.clone(),
                    reason: Some(item.response.response_message),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.transaction_number),
                }),
                ..item.data
            }),
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, TransactionResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, TransactionResponse>,
    ) -> Result<Self, Self::Error> {
        let status = get_status_of_request(item.response.response_code.clone());
        match status {
            true => Ok(Self {
                response: Ok(types::RefundsResponseData {
                    connector_refund_id: item.response.transaction_number,
                    refund_status: common_enums::RefundStatus::Pending,
                }),
                ..item.data
            }),
            false => Ok(Self {
                response: Err(types::ErrorResponse {
                    code: item.response.response_code.clone(),
                    message: item.response.response_message.clone(),
                    reason: Some(item.response.response_message),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.transaction_number),
                }),
                ..item.data
            }),
        }
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PayboxErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            TransactionResponse,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::CompleteAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            TransactionResponse,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response = item.response.clone();
        let status = get_status_of_request(response.response_code.clone());
        match status {
            true => Ok(Self {
                status: match item.data.request.is_auto_capture()? {
                    true => enums::AttemptStatus::Charged,
                    false => enums::AttemptStatus::Authorized,
                },
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        response.paybox_order_id,
                    ),
                    redirection_data: None,
                    mandate_reference: Box::new(None),
                    connector_metadata: Some(serde_json::json!(PayboxMeta {
                        connector_request_id: response.transaction_number.clone()
                    })),
                    network_txn_id: None,
                    connector_response_reference_id: None,
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
                    connector_transaction_id: Some(response.transaction_number),
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedirectionAuthResponse {
    #[serde(rename = "ID3D")]
    three_ds_data: Option<Secret<String>>,
}

impl TryFrom<&PayboxRouterData<&types::PaymentsCompleteAuthorizeRouterData>> for PaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayboxRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let redirect_response = item.router_data.request.redirect_response.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "redirect_response",
            },
        )?;
        let redirect_payload: RedirectionAuthResponse = redirect_response
            .payload
            .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "request.redirect_response.payload",
            })?
            .peek()
            .clone()
            .parse_value("RedirectionAuthResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        match item.router_data.request.payment_method_data.clone() {
            Some(domain::PaymentMethodData::Card(req_card)) => {
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
                    DateFormat::DDMMYYYYHHmmss,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                Ok(Self {
                    date: format_time.clone(),
                    transaction_type,
                    paybox_request_number: get_paybox_request_number()?,
                    amount: item.router_data.request.minor_amount,
                    description_reference: item.router_data.connector_request_reference_id.clone(),
                    version: VERSION_PAYBOX.to_string(),
                    currency,
                    card_number: req_card.card_number,
                    expiration_date,
                    cvv: req_card.card_cvc,
                    activity: PAY_ORIGIN_INTERNET.to_string(),
                    site: auth_data.site,
                    rank: auth_data.rang,
                    key: auth_data.cle,
                    three_ds_data: redirect_payload.three_ds_data.map_or_else(
                        || Some(Secret::new(THREE_DS_FAIL_CODE.to_string())),
                        |data| Some(data.clone()),
                    ),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}
