use api_models::payments::{
    ImmediateBillingType, QrCodeInformation, SantanderBillingType, ScheduledBillingType,
};
use chrono::{DateTime, Utc};
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::Encode,
    pii::Email,
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use crc::{Algorithm, Crc};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, PaymentMethodData},
    router_data::{AccessToken, RouterData},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCancelRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret, WithType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use url::Url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, QrImage, RouterData as _},
};
const CRC_16_CCITT_FALSE: Algorithm<u16> = Algorithm {
    width: 16,
    poly: 0x1021,
    init: 0xFFFF,
    refin: false,
    refout: false,
    xorout: 0x0000,
    check: 0x29B1,
    residue: 0x0000,
};

type Error = error_stack::Report<errors::ConnectorError>;

const DEFAULT_EXPIRATION_TIME: i32 = 3600;
const DEFAULT_VALIDITY_AFTER_EXPIRATION_TIME: i32 = 30;

pub struct SantanderRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for SantanderRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub fn format_emv_field(id: &str, value: &str) -> String {
    format!("{id}{:02}{value}", value.len())
}

pub fn generate_emv_string(
    payload_url: &str,
    merchant_name: &str,
    merchant_city: &str,
    amount: Option<&str>,
    txid: Option<&str>,
) -> String {
    let mut emv = String::new();

    // 00: Payload Format Indicator
    emv += &format_emv_field("00", "01");
    // 01: Point of Initiation Method (dynamic)
    emv += &format_emv_field("01", "12");

    // 26: Merchant Account Info
    let gui = format_emv_field("00", "br.gov.bcb.pix");
    let url = format_emv_field("25", payload_url);
    let merchant_account_info = format_emv_field("26", &(gui + &url));
    emv += &merchant_account_info;

    // 52: Merchant Category Code (0000)
    emv += &format_emv_field("52", "0000");
    // 53: Currency Code (986 for BRL)
    emv += &format_emv_field("53", "986");

    // 54: Amount (optional)
    if let Some(amount) = amount {
        emv += &format_emv_field("54", amount);
    }

    // 58: Country Code (BR)
    emv += &format_emv_field("58", "BR");
    // 59: Merchant Name
    emv += &format_emv_field("59", merchant_name);
    // 60: Merchant City
    emv += &format_emv_field("60", merchant_city);

    // 62: Additional Data Field Template (optional TXID)
    if let Some(txid) = txid {
        let reference = format_emv_field("05", txid);
        emv += &format_emv_field("62", &reference);
    }

    // Placeholder for CRC (we need to calculate this last)
    emv += "6304";

    // Compute CRC16-CCITT (False) checksum
    let crc = Crc::<u16>::new(&CRC_16_CCITT_FALSE);
    let checksum = crc.checksum(emv.as_bytes());
    emv += &format!("{:04X}", checksum);

    emv
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SantanderAuthUpdateResponse {
    #[serde(rename = "camelCase")]
    pub refresh_url: String,
    pub token_type: String,
    pub client_id: String,
    pub access_token: Secret<String>,
    pub scopes: String,
    pub expires_in: i64,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct SantanderCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl<F, T> TryFrom<ResponseRouterData<F, SantanderAuthUpdateResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SantanderAuthUpdateResponse, T, AccessToken>,
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

impl TryFrom<&SantanderRouterData<&PaymentsAuthorizeRouterData>> for SantanderPaymentRequest {
    type Error = Error;
    fn try_from(
        item: &SantanderRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.request.capture_method != Some(enums::CaptureMethod::Automatic) {
            return Err(errors::ConnectorError::FlowNotSupported {
                flow: format!("{:?}", item.router_data.request.capture_method),
                connector: "Santander".to_string(),
            }
            .into());
        }
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankTransfer(ref bank_transfer_data) => {
                Self::try_from((item, bank_transfer_data.as_ref()))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                crate::utils::get_unimplemented_payment_method_error_message("Santander"),
            ))?,
        }
    }
}

pub fn extract_field_from_mca_metadata(
    connector_meta_data: Option<Secret<Value, WithType>>,
    field_name: &str,
) -> Option<Secret<String>> {
    let metadata = connector_meta_data.as_ref()?;

    let exposed = metadata.clone().expose();
    exposed
        .get(field_name)
        .and_then(|v| v.as_str())
        .map(|s| Secret::new(s.to_string()))
}

impl
    TryFrom<(
        &SantanderRouterData<&PaymentsAuthorizeRouterData>,
        &BankTransferData,
    )> for SantanderPaymentRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &SantanderRouterData<&PaymentsAuthorizeRouterData>,
            &BankTransferData,
        ),
    ) -> Result<Self, Self::Error> {
        let chave_key = if let Some(key) =
            extract_field_from_mca_metadata(value.0.router_data.connector_meta_data.clone(), "key")
        {
            key
        } else {
            return Err(errors::ConnectorError::NoConnectorMetaData)?;
        };

        let billing_type = &value.0.router_data.request.santander_pix_qr_expiration_time;

        let (expiration_time, due_date, validity) = match billing_type {
            Some(SantanderBillingType::Immediate(data)) => (data.expiration_time, None, None),
            Some(SantanderBillingType::Scheduled(data)) => {
                (None, data.due_date, data.validity_after_expiration)
            }
            None => (None, None, None),
        };

        let calender = if expiration_time.is_some() {
            SantanderBillingType::Immediate(ImmediateBillingType {
                expiration_time: Some(expiration_time.unwrap_or(DEFAULT_EXPIRATION_TIME)),
            })
        } else if due_date.is_some() {
            SantanderBillingType::Scheduled(ScheduledBillingType {
                due_date,
                validity_after_expiration: Some(
                    validity.unwrap_or(DEFAULT_VALIDITY_AFTER_EXPIRATION_TIME),
                ),
            })
        } else {
            return Err(errors::ConnectorError::MissingRequiredField {
                field_name: "due_date",
            })?;
        };

        let debtor = if expiration_time.is_some() {
            Some(SantanderDebtor::SantanderCob(SantanderCobDebtor {
                cpf: None,
                name: Some(value.0.router_data.get_billing_full_name()?),
            }))
        } else if due_date.is_some() || (expiration_time.is_none() && due_date.is_none()) {
            Some(SantanderDebtor::SantanderCobv(SantanderCobvDebtor {
                email: value.0.router_data.request.get_optional_email(),
                street: value.0.router_data.get_optional_billing_line1(),
                city: value.0.router_data.get_optional_billing_city(),
                uf: None,
                zip_code: value.0.router_data.get_optional_billing_zip(),
                cpj: None,
                name: value.0.router_data.get_optional_billing_full_name(),
            }))
        } else {
            None
        };

        Ok(Self {
            calender,
            debtor,
            value: SantanderValue {
                original: value.0.amount.to_owned(),
            },
            key: chave_key,
            request_payer: value.0.router_data.request.statement_descriptor.clone(),
            additional_info: None,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPaymentRequest {
    #[serde(rename = "calendario")]
    pub calender: SantanderBillingType,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    #[serde(rename = "valor")]
    pub value: SantanderValue,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixQRPaymentRequest {
    #[serde(rename = "calendario")]
    pub calender: SantanderBillingType,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    #[serde(rename = "valor")]
    pub value: SantanderValue,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SantanderDebtor {
    SantanderCob(SantanderCobDebtor),
    SantanderCobv(SantanderCobvDebtor),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderCobDebtor {
    pub cpf: Option<Secret<String>>,
    #[serde(rename = "nome")]
    pub name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderCobvDebtor {
    #[serde(rename = "email")]
    pub email: Option<Email>,
    #[serde(rename = "logradouro")]
    pub street: Option<Secret<String>>,
    #[serde(rename = "cidade")]
    pub city: Option<String>,
    #[serde(rename = "uf")]
    pub uf: Option<Secret<String>>,
    #[serde(rename = "cep")]
    pub zip_code: Option<Secret<String>>,
    pub cpj: Option<Secret<String>>,
    #[serde(rename = "nome")]
    pub name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderValue {
    pub original: StringMajorUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderValueResponse {
    pub original: StringMajorUnit,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderAdditionalInfo {
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "valor")]
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderPaymentStatus {
    Active,
    Completed,
    RemovedByReceivingUser,
    RemovedByPSP,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderVoidStatus {
    RemovedByReceivingUser,
}

impl From<SantanderPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: SantanderPaymentStatus) -> Self {
        match item {
            SantanderPaymentStatus::Active => Self::Authorizing,
            SantanderPaymentStatus::Completed => Self::Charged,
            SantanderPaymentStatus::RemovedByReceivingUser
            | SantanderPaymentStatus::RemovedByPSP => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderPaymentsResponse {
    #[serde(rename = "calendario")]
    pub calendar: SantanderCalendar,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: i32,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    pub location: Option<String>,
    pub status: SantanderPaymentStatus,
    #[serde(rename = "valor")]
    pub value: SantanderValueResponse,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderVoidResponse {
    #[serde(rename = "calendario")]
    pub calendar: SantanderCalendar,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: i32,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    pub location: Option<String>,
    pub status: SantanderPaymentStatus,
    #[serde(rename = "valor")]
    pub value: SantanderValueResponse,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderCalendar {
    #[serde(rename = "calendario")]
    pub creation: DateTime<Utc>,
    #[serde(rename = "expiracao")]
    pub expiration: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderPaymentsSyncResponse {
    pub status: SantanderPaymentStatus,
    pub pix: Vec<SantanderPix>,
    #[serde(rename = "calendario")]
    pub calendar: SantanderCalendar,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    #[serde(rename = "valor")]
    pub value: SantanderValueResponse,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: i32,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPix {
    pub end_to_end_id: Secret<String>,
    #[serde(rename = "txid")]
    pub transaction_id: Secret<String>,
    #[serde(rename = "valor")]
    pub value: String,
    #[serde(rename = "horario")]
    pub time: String,
    #[serde(rename = "infoPagador")]
    pub info_payer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPaymentsCancelRequest {
    pub status: Option<SantanderVoidStatus>,
}

impl<F, T> TryFrom<ResponseRouterData<F, SantanderPaymentsSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SantanderPaymentsSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item.response.clone();
        let connector_metadata = response.pix.first().map(|pix| {
            serde_json::json!({
                "end_to_end_id": pix.end_to_end_id.clone().expose()
            })
        });
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response.transaction_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, SantanderPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SantanderPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item.response.clone();
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response.transaction_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: get_qr_code_data(&item)?,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, SantanderVoidResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SantanderVoidResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item.response.clone();
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response.transaction_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: *Box::new(None),
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<&PaymentsCancelRouterData> for SantanderPaymentsCancelRequest {
    type Error = Error;
    fn try_from(_item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            status: Some(SantanderVoidStatus::RemovedByReceivingUser),
        })
    }
}

fn get_qr_code_data<F, T>(
    item: &ResponseRouterData<F, SantanderPaymentsResponse, T, PaymentsResponseData>,
) -> CustomResult<Option<Value>, errors::ConnectorError> {
    let response = item.response.clone();
    let expiration_time = item.response.calendar.expiration;

    let expiration_i64 = i64::from(expiration_time);

    let rfc3339_expiry = (OffsetDateTime::now_utc() + time::Duration::seconds(expiration_i64))
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?;

    let qr_expiration_duration = OffsetDateTime::parse(
        rfc3339_expiry.as_str(),
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?
    .unix_timestamp()
        * 1000;

    let connector_metadata = item.data.connector_meta_data.clone();

    let merchant_city_value =
        extract_field_from_mca_metadata(connector_metadata.clone(), "merchant_city")
            .map(|city| city.expose())
            .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                field_name: "merchant_city",
            })?;
    let merchant_city = merchant_city_value.as_str();

    let merchant_name_value = extract_field_from_mca_metadata(connector_metadata, "merchant_name")
        .map(|name| name.expose())
        .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
            field_name: "merchant_name",
        })?;
    let merchant_name = merchant_name_value.as_str();

    let payload_url = if let Some(location) = response.location {
        location
    } else {
        return Err(errors::ConnectorError::ResponseHandlingFailed)?;
    };

    let amount_i64 = StringMajorUnitForConnector
        .convert_back(response.value.original, enums::Currency::BRL)
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?
        .get_amount_as_i64();

    let amount_string = amount_i64.to_string();
    let amount = amount_string.as_str();

    let dynamic_pix_code = generate_emv_string(
        payload_url.as_str(),
        merchant_name,
        merchant_city,
        Some(amount),
        Some(response.transaction_id.as_str()),
    );

    let image_data = QrImage::new_from_data(dynamic_pix_code.clone())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let image_data_url = Url::parse(image_data.data.clone().as_str())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let qr_code_info = QrCodeInformation::QrDataUrl {
        image_data_url,
        display_to_timestamp: Some(qr_expiration_duration),
    };

    Some(qr_code_info.encode_to_value())
        .transpose()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

#[derive(Default, Debug, Serialize)]
pub struct SantanderRefundRequest {
    #[serde(rename = "valor")]
    pub value: StringMajorUnit,
}

impl<F> TryFrom<&SantanderRouterData<&RefundsRouterData<F>>> for SantanderRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SantanderRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            value: item.amount.to_owned(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderRefundStatus {
    InProcessing,
    Returned,
    NotDone,
}

impl From<SantanderRefundStatus> for enums::RefundStatus {
    fn from(item: SantanderRefundStatus) -> Self {
        match item {
            SantanderRefundStatus::Returned => Self::Success,
            SantanderRefundStatus::NotDone => Self::Failure,
            SantanderRefundStatus::InProcessing => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderRefundResponse {
    pub id: Secret<String>,
    pub rtr_id: Secret<String>,
    #[serde(rename = "valor")]
    pub value: StringMajorUnit,
    #[serde(rename = "horario")]
    pub time: SantanderTime,
    pub status: SantanderRefundStatus,
    #[serde(rename = "motivo")]
    pub reason: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderTime {
    #[serde(rename = "solicitacao")]
    pub request: Option<String>,
    #[serde(rename = "liquidacao")]
    pub liquidation: Option<String>,
}

impl<F> TryFrom<RefundsResponseRouterData<F, SantanderRefundResponse>> for RefundsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<F, SantanderRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.rtr_id.clone().expose(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderErrorResponse {
    #[serde(rename = "type")]
    pub field_type: Secret<String>,
    pub title: String,
    pub status: i64,
    pub detail: Option<String>,
    pub correlation_id: Option<String>,
    #[serde(rename = "violacoes")]
    pub violations: Option<Vec<SantanderViolations>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SantanderViolations {
    #[serde(rename = "razao")]
    pub reason: Option<String>,
    #[serde(rename = "propriedade")]
    pub property: Option<String>,
    #[serde(rename = "valor")]
    pub value: Option<String>,
}
