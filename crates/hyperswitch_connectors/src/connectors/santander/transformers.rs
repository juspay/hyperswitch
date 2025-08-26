use api_models::payments::{QrCodeInformation, VoucherNextStepData};
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, Encode},
    id_type,
    types::{AmountConvertor, FloatMajorUnit, StringMajorUnit, StringMajorUnitForConnector},
};
use crc::{Algorithm, Crc};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, PaymentMethodData, VoucherData},
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCancelRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;
// use time::OffsetDateTime;
use url::Url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self as connector_utils, QrImage, RouterData as _},
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
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderMetadataObject {
    pub pix_key: Secret<String>,
    // pub expiration_time: PixQRExpirationDuration,
    pub cpf: Secret<String>,
    pub merchant_city: String,
    pub merchant_name: String,
    pub workspace_id: String,
    pub covenant_code: String,
}

// #[derive(Debug, Serialize, Deserialize)]
// pub enum PixQRExpirationDuration {
//     Immediate(ImmediateExpirationTime),
//     Scheduled(DueDateExpirationTime),
// }

// #[derive(Debug, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct ImmediateExpirationTime {
//     pub time_in_seconds: i32, // in seconds
// }

// #[derive(Debug, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct DueDateExpirationTime {
//     pub date: chrono::NaiveDate,                // in YYYY-MM-DD format
//     pub validity_after_expiration: Option<i32>, // in days
// }

impl TryFrom<&Option<common_utils::pii::SecretSerdeValue>> for SantanderMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        meta_data: &Option<common_utils::pii::SecretSerdeValue>,
    ) -> Result<Self, Self::Error> {
        let metadata = connector_utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
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
    emv += &format!("{checksum:04X}");

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

#[derive(Default, Debug, Serialize)]
pub struct SantanderCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

pub struct SantanderAuthType {
    pub(super) _api_key: Secret<String>,
    pub(super) _key1: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SantanderAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                _api_key: api_key.to_owned(),
                _key1: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
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

impl
    TryFrom<(
        &SantanderRouterData<&PaymentsAuthorizeRouterData>,
        &router_env::env::Env,
    )> for SantanderPaymentRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &SantanderRouterData<&PaymentsAuthorizeRouterData>,
            &router_env::env::Env,
        ),
    ) -> Result<Self, Self::Error> {
        if value.0.router_data.request.capture_method != Some(enums::CaptureMethod::Automatic) {
            return Err(errors::ConnectorError::FlowNotSupported {
                flow: format!("{:?}", value.0.router_data.request.capture_method),
                connector: "Santander".to_string(),
            }
            .into());
        }
        match value.0.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankTransfer(ref bank_transfer_data) => {
                Self::try_from((value.0, bank_transfer_data.as_ref()))
            }
            PaymentMethodData::Voucher(ref voucher_data) => {
                Self::try_from((value.0, voucher_data, value.1))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                crate::utils::get_unimplemented_payment_method_error_message("Santander"),
            ))?,
        }
    }
}

impl
    TryFrom<(
        &SantanderRouterData<&PaymentsAuthorizeRouterData>,
        &VoucherData,
        &router_env::env::Env,
    )> for SantanderPaymentRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &SantanderRouterData<&PaymentsAuthorizeRouterData>,
            &VoucherData,
            &router_env::env::Env,
        ),
    ) -> Result<Self, Self::Error> {
        let santander_mca_metadata =
            SantanderMetadataObject::try_from(&value.0.router_data.connector_meta_data)?;

        let voucher_data = match &value.0.router_data.request.payment_method_data {
            PaymentMethodData::Voucher(VoucherData::Boleto(boleto_data)) => boleto_data,
            _ => {
                return Err(errors::ConnectorError::NotImplemented(
                    crate::utils::get_unimplemented_payment_method_error_message("Santander"),
                )
                .into());
            }
        };

        Ok(Self::Boleto(Box::new(SantanderBoletoPaymentRequest {
            environment: Environment::from(*value.2),
            nsu_code: value.0.router_data.payment_id.clone(), // size: 20
            nsu_date: time::OffsetDateTime::now_utc()
                .date()
                .format(&time::macros::format_description!("[year]-[month]-[day]"))
                .change_context(errors::ConnectorError::DateFormattingFailed)?,
            covenant_code: santander_mca_metadata.covenant_code.clone(), // size: 9
            bank_number: voucher_data.bank_number.clone().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "document_type",
                }
            })?, // size: 13
            client_number: Some(value.0.router_data.get_customer_id()?),
            due_date: voucher_data.due_date.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "due_date",
                },
            )?, // format: YYYY-MM-DD
            issue_date: time::OffsetDateTime::now_utc()
                .date()
                .format(&time::macros::format_description!("[year]-[month]-[day]"))
                .change_context(errors::ConnectorError::DateFormattingFailed)?,
            currency: Some(value.0.router_data.request.currency),
            nominal_value: value.0.amount.to_owned(),
            participant_code: value
                .0
                .router_data
                .request
                .merchant_order_reference_id
                .clone(),
            payer: Payer {
                name: value.0.router_data.get_billing_full_name()?,
                document_type: voucher_data.document_type.ok_or_else(|| {
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "document_type",
                    }
                })?,
                document_number: voucher_data.social_security_number.clone(),
                address: Secret::new(
                    [
                        value.0.router_data.get_billing_line1()?,
                        value.0.router_data.get_billing_line2()?,
                    ]
                    .map(|s| s.expose())
                    .join(" "),
                ),
                neighborhood: value.0.router_data.get_billing_line1()?,
                city: value.0.router_data.get_billing_city()?,
                state: value.0.router_data.get_billing_state()?,
                zipcode: value.0.router_data.get_billing_zip()?,
            },
            beneficiary: None,
            document_kind: BoletoDocumentKind::BillProposal, // to change
            discount: Some(Discount {
                discount_type: DiscountType::Free, // to change
                discount_one: None,
                discount_two: None,
                discount_three: None,
            }),
            fine_percentage: voucher_data.fine_percentage.clone(),
            fine_quantity_days: voucher_data.fine_quantity_days.clone(),
            interest_percentage: voucher_data.interest_percentage.clone(),
            deduction_value: None,
            protest_type: None,
            protest_quantity_days: None,
            write_off_quantity_days: voucher_data.write_off_quantity_days.clone(),
            payment_type: PaymentType::Registration,
            parcels_quantity: None,
            value_type: None,
            min_value_or_percentage: None,
            max_value_or_percentage: None,
            iof_percentage: None,
            sharing: None,
            key: None,
            tx_id: None,
            messages: voucher_data.messages.clone(),
        })))
    }
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
        let santander_mca_metadata =
            SantanderMetadataObject::try_from(&value.0.router_data.connector_meta_data)?;

        let debtor = Some(SantanderDebtor {
            cpf: santander_mca_metadata.cpf.clone(),
            name: value.0.router_data.get_billing_full_name()?,
            email: value.0.router_data.get_optional_billing_email(),
            street: value.0.router_data.get_optional_billing_line1(),
            city: value.0.router_data.get_optional_billing_city(),
            uf: value.0.router_data.get_billing_state()?,
            zip_code: value.0.router_data.get_optional_billing_zip(),
        });

        let calendar = match &value
            .0
            .router_data
            .request
            .feature_metadata
            .as_ref()
            .and_then(|f| f.pix_qr_expiry_time.as_ref())
        {
            Some(api_models::payments::PixQRExpirationDuration::Immediate(val)) => {
                SantanderPixCalendar::Immediate(SantanderPixImmediateCalendar {
                    expiration: val.time,
                })
            }
            Some(api_models::payments::PixQRExpirationDuration::Scheduled(val)) => {
                SantanderPixCalendar::Scheduled(SantanderPixDueDateCalendar {
                    due_date: val.date.clone(),
                    validity_after_expiration: val.validity_after_expiration,
                })
            }
            None => {
                SantanderPixCalendar::Immediate(SantanderPixImmediateCalendar {
                    expiration: 3600, // default 1 hour
                })
            }
        };

        Ok(Self::PixQR(Box::new(SantanderPixQRPaymentRequest {
            calendar,
            debtor,
            value: SantanderValue {
                original: value.0.amount.to_owned(),
            },
            key: santander_mca_metadata.pix_key.clone(),
            request_payer: value.0.router_data.request.statement_descriptor.clone(),
            additional_info: None,
        })))
    }
}

#[derive(Debug, Serialize)]
pub enum SantanderPaymentRequest {
    PixQR(Box<SantanderPixQRPaymentRequest>),
    Boleto(Box<SantanderBoletoPaymentRequest>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discount {
    #[serde(rename = "type")]
    pub discount_type: DiscountType,
    pub discount_one: Option<DiscountObject>,
    pub discount_two: Option<DiscountObject>,
    pub discount_three: Option<DiscountObject>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoPaymentRequest {
    // pub workspace_id: String,
    pub environment: Environment,
    pub nsu_code: String,
    pub nsu_date: String,
    pub covenant_code: String,
    pub bank_number: String,
    pub client_number: Option<id_type::CustomerId>,
    pub due_date: String,
    pub issue_date: String,
    pub currency: Option<enums::Currency>,
    pub nominal_value: StringMajorUnit,
    pub participant_code: Option<String>,
    pub payer: Payer,
    pub beneficiary: Option<Beneficiary>,
    pub document_kind: BoletoDocumentKind,
    pub discount: Option<Discount>,
    pub fine_percentage: Option<String>,
    pub fine_quantity_days: Option<String>,
    pub interest_percentage: Option<String>,
    pub deduction_value: Option<FloatMajorUnit>,
    pub protest_type: Option<ProtestType>,
    pub protest_quantity_days: Option<i64>,
    pub write_off_quantity_days: Option<String>,
    pub payment_type: PaymentType,
    pub parcels_quantity: Option<i64>,
    pub value_type: Option<String>,
    pub min_value_or_percentage: Option<f64>,
    pub max_value_or_percentage: Option<f64>,
    pub iof_percentage: Option<f64>,
    pub sharing: Option<Sharing>,
    pub key: Option<Key>,
    pub tx_id: Option<String>,
    pub messages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payer {
    pub name: Secret<String>,
    pub document_type: enums::DocumentKind,
    pub document_number: Option<Secret<String>>,
    pub address: Secret<String>,
    pub neighborhood: Secret<String>,
    pub city: String,
    pub state: Secret<String>,
    pub zipcode: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Beneficiary {
    pub name: Option<Secret<String>>,
    pub document_type: Option<enums::DocumentKind>,
    pub document_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Environment {
    #[serde(rename = "Teste")]
    Sandbox,
    #[serde(rename = "Producao")]
    Production,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SantanderDocumentKind {
    Cpf,
    Cnpj,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoletoDocumentKind {
    #[serde(rename = "DUPLICATA_MERCANTIL")]
    DuplicateMercantil,
    #[serde(rename = "DUPLICATA_SERVICO")]
    DuplicateService,
    #[serde(rename = "NOTA_PROMISSORIA")]
    PromissoryNote,
    #[serde(rename = "NOTA_PROMISSORIA_RURAL")]
    RuralPromissoryNote,
    #[serde(rename = "RECIBO")]
    Receipt,
    #[serde(rename = "APOLICE_SEGURO")]
    InsurancePolicy,
    #[serde(rename = "BOLETO_CARTAO_CREDITO")]
    BillCreditCard,
    #[serde(rename = "BOLETO_PROPOSTA")]
    BillProposal,
    #[serde(rename = "BOLETO_DEPOSITO_APORTE")]
    BoletoDepositoAponte,
    #[serde(rename = "CHEQUE")]
    Check,
    #[serde(rename = "NOTA_PROMISSORIA_DIRETA")]
    DirectPromissoryNote,
    #[serde(rename = "OUTROS")]
    Others,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscountType {
    #[serde(rename = "ISENTO")]
    Free,
    #[serde(rename = "VALOR_DATA_FIXA")]
    FixedDateValue,
    #[serde(rename = "VALOR_DIA_CORRIDO")]
    ValueDayConductor,
    #[serde(rename = "VALOR_DIA_UTIL")]
    ValueWorthDay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct DiscountObject {
    pub value: StringMajorUnit, // Changed from f64 → String for API safety
    pub limit_date: String,     // YYYY-MM-DD
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProtestType {
    #[serde(rename = "SEM_PROTESTO")]
    WithoutProtest,

    #[serde(rename = "DIAS_CORRIDOS")]
    DaysConducted,

    #[serde(rename = "DIAS_UTEIS")]
    WorkingDays,

    #[serde(rename = "CADASTRO_CONVENIO")]
    RegistrationAgreement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaymentType {
    #[serde(rename = "REGISTRO")]
    Registration,

    #[serde(rename = "DIVERGENTE")]
    Divergent,

    #[serde(rename = "PARCIAL")]
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sharing {
    pub code: String,
    pub value: FloatMajorUnit, // Changed from f64 → String for monetary precision
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    #[serde(rename = "type")]
    pub key_type: Option<String>,
    pub dict_key: Option<String>,
}

// #[derive(Debug, Serialize)]
// #[serde(rename_all = "camelCase")]
// pub struct SantanderPixQRCodeRequest {
//     #[serde(rename = "calendario")]
//     pub calender: SantanderCalendar,
//     #[serde(rename = "devedor")]
//     pub debtor: SantanderDebtor,
//     #[serde(rename = "valor")]
//     pub value: SantanderValue,
//     #[serde(rename = "chave")]
//     pub key: Secret<String>,
//     #[serde(rename = "solicitacaoPagador")]
//     pub request_payer: Option<String>,
//     #[serde(rename = "infoAdicionais")]
//     pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
// }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixQRPaymentRequest {
    #[serde(rename = "calendario")]
    pub calendar: SantanderPixCalendar,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderDebtor {
    #[serde(rename = "email")]
    pub email: Option<common_utils::pii::Email>,
    #[serde(rename = "logradouro")]
    pub street: Option<Secret<String>>,
    #[serde(rename = "cidade")]
    pub city: Option<String>,
    #[serde(rename = "uf")]
    pub uf: Secret<String>,
    #[serde(rename = "cep")]
    pub zip_code: Option<Secret<String>>,
    #[serde(rename = "cpf")]
    pub cpf: Secret<String>,
    #[serde(rename = "nome")]
    pub name: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderValue {
    pub original: StringMajorUnit,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderAdditionalInfo {
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "valor")]
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SantanderPaymentStatus {
    Active,
    Completed,
    RemovedByReceivingUser,
    RemovedByPSP,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

impl From<router_env::env::Env> for Environment {
    fn from(item: router_env::env::Env) -> Self {
        match item {
            router_env::env::Env::Sandbox | router_env::env::Env::Development => Self::Sandbox,
            router_env::env::Env::Production => Self::Production,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SantanderPaymentsResponse {
    PixQRCode(Box<SantanderPixQRCodePaymentsResponse>),
    Boleto(Box<SantanderBoletoPaymentsResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderBoletoPaymentsResponse {
    pub environment: Environment,
    pub nsu_code: String,
    pub nsu_date: String,
    pub covenant_code: String,
    pub bank_number: String,
    pub client_number: Option<id_type::CustomerId>,
    pub due_date: String,
    pub issue_date: String,
    pub participant_code: Option<String>,
    pub nominal_value: StringMajorUnit,
    pub payer: Payer,
    pub beneficiary: Option<Beneficiary>,
    pub document_kind: BoletoDocumentKind,
    pub discount: Option<Discount>,
    pub fine_percentage: Option<String>,
    pub fine_quantity_days: Option<String>,
    pub interest_percentage: Option<String>,
    pub deduction_value: Option<FloatMajorUnit>,
    pub protest_type: Option<ProtestType>,
    pub protest_quantity_days: Option<i64>,
    pub write_off_quantity_days: Option<String>,
    pub payment_type: PaymentType,
    pub parcels_quantity: Option<i64>,
    pub value_type: Option<String>,
    pub min_value_or_percentage: Option<f64>,
    pub max_value_or_percentage: Option<f64>,
    pub iof_percentage: Option<f64>,
    pub sharing: Option<Sharing>,
    pub key: Option<Key>,
    pub tx_id: Option<String>,
    pub messages: Option<Vec<String>>,
    pub barcode: Option<String>,
    pub digitable_line: Option<String>,
    pub entry_date: Option<String>,
    pub qr_code_pix: Option<String>,
    pub qr_code_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SantanderPixResponseCalendar {
    Immediate(SantanderPixImmediateResponseCalendar),
    Scheduled(SantanderPixDueDateResponseCalendar),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixDueDateResponseCalendar {
    pub creation: String,
    pub due_date: String,
    pub validity_after_expiration: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixImmediateResponseCalendar {
    pub creation: String,
    pub expiration: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixQRCodePaymentsResponse {
    #[serde(rename = "calendario")]
    pub calendar: SantanderPixResponseCalendar,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: i32,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    pub location: Option<String>,
    pub status: SantanderPaymentStatus,
    #[serde(rename = "valor")]
    pub value: SantanderValue,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderVoidResponse {
    #[serde(rename = "calendario")]
    pub calendar: SantanderPixResponseCalendar,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: i32,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    pub location: Option<String>,
    pub status: SantanderPaymentStatus,
    #[serde(rename = "valor")]
    pub value: SantanderValue,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SantanderPixCalendar {
    Immediate(SantanderPixImmediateCalendar),
    Scheduled(SantanderPixDueDateCalendar),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixImmediateCalendar {
    // #[serde(rename = "calendario")]
    // pub creation: String,
    #[serde(rename = "expiracao")]
    pub expiration: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixDueDateCalendar {
    #[serde(rename = "dataDeVencimento")]
    pub due_date: String,
    #[serde(rename = "validadeAposVencimento")]
    pub validity_after_expiration: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SantanderPaymentsSyncResponse {
    PixQRCode(Box<SantanderPixQRCodeResponse>),
    Boleto(Box<SantanderBoletoPaymentsResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixQRCodeResponse {
    pub status: SantanderPaymentStatus,
    pub pix: Vec<SantanderPix>,
    #[serde(rename = "calendario")]
    pub calendar: SantanderPixResponseCalendar,
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
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: i32,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

        match response {
            SantanderPaymentsSyncResponse::PixQRCode(pix_data) => {
                let connector_metadata = pix_data.pix.first().map(|pix| {
                    serde_json::json!({
                        "end_to_end_id": pix.end_to_end_id.clone().expose()
                    })
                });
                Ok(Self {
                    status: common_enums::AttemptStatus::from(pix_data.status),
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            pix_data.transaction_id.clone(),
                        ),
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
            SantanderPaymentsSyncResponse::Boleto(boleto_data) => {
                let voucher_data = VoucherNextStepData {
                    expires_at: None,
                    digitable_line: boleto_data.digitable_line,
                    reference: boleto_data.barcode.ok_or(
                        errors::ConnectorError::MissingConnectorRedirectionPayload {
                            field_name: "barcode",
                        },
                    )?,
                    entry_date: boleto_data.entry_date,
                    download_url: None,
                    instructions_url: None,
                };

                let connector_metadata = Some(voucher_data.encode_to_value())
                    .transpose()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

                let resource_id = match boleto_data.tx_id {
                    Some(tx_id) => ResponseId::ConnectorTransactionId(tx_id),
                    None => ResponseId::NoResponseId,
                };

                Ok(Self {
                    status: common_enums::AttemptStatus::AuthenticationPending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id,
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata,
                        network_txn_id: None,
                        connector_response_reference_id: Some(boleto_data.bank_number),
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    ..item.data
                })
            }
        }
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

        match response {
            SantanderPaymentsResponse::PixQRCode(pix_data) => Ok(Self {
                status: common_enums::AttemptStatus::from(pix_data.status.clone()),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        pix_data.transaction_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: get_qr_code_data(&item, &pix_data)?,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            SantanderPaymentsResponse::Boleto(boleto_data) => {
                let voucher_data = VoucherNextStepData {
                    expires_at: None,
                    digitable_line: boleto_data.digitable_line,
                    reference: boleto_data.barcode.ok_or(
                        errors::ConnectorError::MissingConnectorRedirectionPayload {
                            field_name: "barcode",
                        },
                    )?,
                    entry_date: boleto_data.entry_date,
                    download_url: None,
                    instructions_url: None,
                };

                let connector_metadata = Some(voucher_data.encode_to_value())
                    .transpose()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

                let resource_id = match boleto_data.tx_id {
                    Some(tx_id) => ResponseId::ConnectorTransactionId(tx_id),
                    None => ResponseId::NoResponseId,
                };

                Ok(Self {
                    status: common_enums::AttemptStatus::AuthenticationPending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id,
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata,
                        network_txn_id: None,
                        connector_response_reference_id: Some(boleto_data.bank_number),
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    ..item.data
                })
            }
        }
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
    pix_data: &SantanderPixQRCodePaymentsResponse,
) -> CustomResult<Option<Value>, errors::ConnectorError> {
    let santander_mca_metadata = SantanderMetadataObject::try_from(&item.data.connector_meta_data)?;

    let response = pix_data.clone();

    let merchant_city = santander_mca_metadata.merchant_city.as_str();

    let merchant_name = santander_mca_metadata.merchant_name.as_str();

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
        display_to_timestamp: None,
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
pub enum SantanderErrorResponse {
    PixQrCode(SantanderPixQRCodeErrorResponse),
    Boleto(SantanderBoletoErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderBoletoErrorResponse {
    #[serde(rename = "_errorCode")]
    pub error_code: i64,

    #[serde(rename = "_message")]
    pub error_message: String,

    #[serde(rename = "_details")]
    pub issuer_error_message: String,

    #[serde(rename = "_timestamp")]
    pub timestamp: String,

    #[serde(rename = "_traceId")]
    pub trace_id: String,

    #[serde(rename = "_errors")]
    pub errors: Option<Vec<ErrorObject>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    #[serde(rename = "_code")]
    pub code: Option<i64>,

    #[serde(rename = "_field")]
    pub field: Option<String>,

    #[serde(rename = "_message")]
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixQRCodeErrorResponse {
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderWebhookBody {
    pub message: MessageCode,   // meaning of this enum variant is not clear
    pub function: FunctionType, // event type of the webhook
    pub payment_type: WebhookPaymentType,
    pub issue_date: String,
    pub payment_date: String,
    pub bank_code: String,
    pub payment_channel: PaymentChannel,
    pub payment_kind: PaymentKind,
    pub covenant: String,
    pub type_of_person_agreement: enums::DocumentKind,
    pub agreement_document: String,
    pub bank_number: String,
    pub client_number: String,
    pub participant_code: String,
    pub tx_id: String,
    pub payer_document_type: enums::DocumentKind,
    pub payer_document_number: String,
    pub payer_name: String,
    pub final_beneficiaryr_document_type: enums::DocumentKind,
    pub final_beneficiary_document_number: String,
    pub final_beneficiary_name: String,
    pub due_date: String,
    pub nominal_value: StringMajorUnit,
    pub payed_value: String,
    pub interest_value: String,
    pub fine: String,
    pub deduction_value: String,
    pub rebate_value: String,
    pub iof_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageCode {
    Wbhkpagest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionType {
    Pagamento, // Payment
    Estorno,   // Refund
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebhookPaymentType {
    Santander,
    OutrosBancos,
    Pix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// Represents the channel through which a boleto payment was made.
pub enum PaymentChannel {
    /// Payment made at a bank branch or ATM (self-service).
    AgenciasAutoAtendimento,

    /// Payment made through online banking.
    InternetBanking,

    /// Payment made at a physical correspondent agent (e.g., convenience stores, partner outlets).
    CorrespondenteBancarioFisico,

    /// Payment made via Santander’s call center.
    CentralDeAtendimento,

    /// Payment made via electronic file, typically for bulk company payments.
    ArquivoEletronico,

    /// Payment made via DDA (Débito Direto Autorizado) / electronic bill presentment system.
    Dda,

    /// Payment made via digital correspondent channels (apps, kiosks, digital partners).
    CorrespondenteBancarioDigital,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// Represents the type of payment instrument used to pay a boleto.
pub enum PaymentKind {
    /// Payment made in cash or physical form (not via account or card).
    Especie,

    /// Payment made via direct debit from a bank account.
    DebitoEmConta,

    /// Payment made via credit card.
    CartaoDeCredito,

    /// Payment made via check.
    Cheque,
}

pub(crate) fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<SantanderWebhookBody, common_utils::errors::ParsingError> {
    let webhook: SantanderWebhookBody = body.parse_struct("SantanderIncomingWebhook")?;

    Ok(webhook)
}

pub(crate) fn get_santander_webhook_event(
    event_type: FunctionType,
) -> api_models::webhooks::IncomingWebhookEvent {
    // need to confirm about the other possible webhook event statues, as of now only two known
    match event_type {
        FunctionType::Pagamento => api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess,
        FunctionType::Estorno => api_models::webhooks::IncomingWebhookEvent::RefundSuccess,
    }
}
