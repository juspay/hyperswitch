use api_models::payments::{QrCodeInformation, VoucherNextStepData};
use chrono::Utc;
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, Encode},
    id_type,
    types::{
        AmountConvertor, FloatMajorUnit, MinorUnit, StringMajorUnit, StringMajorUnitForConnector,
    },
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
use time::OffsetDateTime;
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
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderMetadataObject {
    pub pix_key: Secret<String>,
    pub expiration_time: i32,
    pub cpf: Secret<String>,
    pub merchant_city: String,
    pub merchant_name: String,
    pub workspace_id: String,
    pub covenant_code: String,
}

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

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
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
            workspace_id: santander_mca_metadata.workspace_id.clone(),
            environment: Environment::from(value.2.clone()),
            nsu_code: value.0.router_data.payment_id.clone(), // size: 20
            nsu_date: Utc::now().date_naive(),
            covenant_code: santander_mca_metadata.covenant_code.clone(), // size: 9
            bank_number: voucher_data.bank_number.clone().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "document_type",
                }
            })?, // size: 13
            client_number: Some(value.0.router_data.get_customer_id()?),
            due_date: voucher_data.due_date.ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "due_date",
                },
            )?, // format: YYYY-MM-DD
            issue_date: Utc::now().date_naive(),
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
            document_kind: BoletoDocumentKind::BoletoProposta, // to change
<<<<<<< Updated upstream
            discount: DiscountType::Isento,                    // to change
=======
            discount: DiscountType::Isento,
>>>>>>> Stashed changes
            discount_one: None,
            discount_two: None,
            discount_three: None,
            fine_percentage: voucher_data.fine_percentage,
            fine_quantity_days: voucher_data.fine_quantity_days,
            interest_percentage: voucher_data.interest_percentage,
            deduction_value: None,
            protest_type: None,
            protest_quantity_days: None,
            write_off_quantity_days: voucher_data.write_off_quantity_days,
            payment_type: PaymentType::Registro,
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
        });

        Ok(Self::PixQR(Box::new(SantanderPixQRPaymentRequest {
            calender: SantanderCalendar {
                creation: Utc::now().to_rfc3339(),
                expiration: santander_mca_metadata.expiration_time,
            },
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoPaymentRequest {
    pub workspace_id: String,
    pub environment: Environment,
    pub nsu_code: String,
    pub nsu_date: chrono::NaiveDate,
    pub covenant_code: String,
    pub bank_number: String,
    pub client_number: Option<id_type::CustomerId>,
    pub due_date: chrono::NaiveDate,
    pub issue_date: chrono::NaiveDate,
    pub currency: Option<enums::Currency>,
    pub nominal_value: StringMajorUnit,
    pub participant_code: Option<String>,
    pub payer: Payer,
    pub beneficiary: Option<Beneficiary>,
    pub document_kind: BoletoDocumentKind,
    pub discount: DiscountType,
    pub discount_one: Option<Discount>,
    pub discount_two: Option<Discount>,
    pub discount_three: Option<Discount>,
    pub fine_percentage: Option<FloatMajorUnit>,
    pub fine_quantity_days: Option<MinorUnit>,
    pub interest_percentage: Option<FloatMajorUnit>,
    pub deduction_value: Option<FloatMajorUnit>,
    pub protest_type: Option<ProtestType>,
    pub protest_quantity_days: Option<MinorUnit>,
    pub write_off_quantity_days: Option<MinorUnit>,
    pub payment_type: PaymentType,
    pub parcels_quantity: Option<MinorUnit>,
    pub value_type: Option<String>,
    pub min_value_or_percentage: Option<FloatMajorUnit>,
    pub max_value_or_percentage: Option<FloatMajorUnit>,
    pub iof_percentage: Option<FloatMajorUnit>,
    pub sharing: Option<Sharing>,
    pub key: Option<Key>,
    pub tx_id: Option<String>,
    pub messages: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Beneficiary {
    pub name: Option<Secret<String>>,
    pub document_type: Option<enums::DocumentKind>,
    pub document_number: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Environment {
    Teste,
    Producao,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SantanderDocumentKind {
    Cpf,
    Cnpj,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoletoDocumentKind {
    DuplicataMercantil,
    DuplicataServico,
    NotaPromissoria,
    NotaPromissoriaRural,
    Recibo,
    ApoliceSeguro,
    BoletoCartaoCredito,
    BoletoProposta,
    BoletoDepositoAporte,
    Cheque,
    NotaPromissoriaDireta,
    Outros,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscountType {
    Isento,
    ValorDataFixa,
    ValorDiaCorrido,
    ValorDiaUtil,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Discount {
    pub value: StringMajorUnit, // Changed from f64 → String for API safety
    pub limit_date: String,     // YYYY-MM-DD
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProtestType {
    SemProtesto,
    DiasCorridos,
    DiasUteis,
    CadastroConvenio,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaymentType {
    Registro,
    Divergente,
    Parcial,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sharing {
    pub code: String,
    pub value: FloatMajorUnit, // Changed from f64 → String for monetary precision
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Key {
    #[serde(rename = "type")]
    pub key_type: Option<String>,
    pub dict_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixQRCodeRequest {
    #[serde(rename = "calendario")]
    pub calender: SantanderCalendar,
    #[serde(rename = "devedor")]
    pub debtor: SantanderDebtor,
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
    pub calender: SantanderCalendar,
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
pub struct SantanderDebtor {
    pub cpf: Secret<String>,
    #[serde(rename = "nome")]
    pub name: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SantanderValue {
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

impl From<router_env::env::Env> for Environment {
    fn from(item: router_env::env::Env) -> Self {
        match item {
            router_env::env::Env::Sandbox | router_env::env::Env::Development => Self::Teste,
            router_env::env::Env::Production => Self::Producao,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SantanderPaymentsResponse {
    PixQRCode(SantanderPixQRCodePaymentsResponse),
    Boleto(SantanderBoletoPaymentsResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderBoletoPaymentsResponse {
    barcode: String,        // data type not clear
    digitable_line: String, // data type not clear
    entry_date: i64,        // data type not clear
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderPixQRCodePaymentsResponse {
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
    pub value: SantanderValue,
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
    pub creation: String,
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
                    digitable_line: Some(boleto_data.digitable_line),
                    reference: boleto_data.barcode,
                    entry_date: Some(boleto_data.entry_date),
                    download_url: None,
                    instructions_url: None,
                };

                let connector_metadata = Some(voucher_data.encode_to_value())
                    .transpose()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

                Ok(Self {
                    status: common_enums::AttemptStatus::AuthenticationPending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::NoResponseId,
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
    let expiration_time = response.calendar.expiration;

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
pub enum SantanderErrorResponse {
    PixQrCode(SantanderPixQRCodeErrorResponse),
    Boleto(SantanderBoletoErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderBoletoErrorResponse {
    #[serde(rename = "_errorCode")]
    pub error_code: String,

    #[serde(rename = "_message")]
    pub error_message: String,

    #[serde(rename = "_details")]
    pub issuer_error_message: String,

    #[serde(rename = "_timestamp")]
    pub timestamp: String,

    #[serde(rename = "_traceId")]
    pub trace_id: String,

    #[serde(skip_serializing, skip_deserializing)]
    pub errors: Option<String>,
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
    pub issue_date: chrono::NaiveDate,
    pub payment_date: chrono::NaiveDate,
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
    pub nominal_value: String,
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
