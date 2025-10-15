use std::collections::HashMap;

use api_models::payments::{QrCodeInformation, VoucherNextStepData};
use common_enums::{enums, AttemptStatus};
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, Encode},
    id_type,
    request::Method,
    types::{AmountConvertor, FloatMajorUnit, StringMajorUnit, StringMajorUnitForConnector},
};
use crc::{Algorithm, Crc};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, PaymentMethodData, VoucherData},
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsSyncRouterData,
        PaymentsUpdateMetadataRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors::{self},
};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::{
    types::{RefreshTokenRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{self as connector_utils, QrImage, RouterData as _},
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
    pub cpf: Secret<String>,        // req in scheduled type pix
    pub cnpj: Secret<String>,        // req in immediate type pix
    pub merchant_city: String,
    pub merchant_name: String,
    pub workspace_id: String,
    pub covenant_code: String, // max_size : 9
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoUpdateRequest {
    #[serde(skip_deserializing)]
    pub covenant_code: String,
    #[serde(skip_deserializing)]
    pub bank_number: String,
    pub due_date: Option<String>,
    pub discount: Option<Discount>,
    pub min_value_or_percentage: Option<f64>,
    pub max_value_or_percentage: Option<f64>,
    pub interest: Option<InterestPercentage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestPercentage {
    pub interest_percentage: String,
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

impl TryFrom<&PaymentsUpdateMetadataRouterData> for SantanderBoletoUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsUpdateMetadataRouterData) -> Result<Self, Self::Error> {
        let update_metadata_fields = validate_metadata_fields(&item.request.metadata.clone())?;

        let santander_mca_metadata = SantanderMetadataObject::try_from(&item.connector_meta_data)?;

        Ok(Self {
            covenant_code: santander_mca_metadata.covenant_code,
            bank_number: extract_bank_number(item.request.connector_meta.clone())?,
            due_date: update_metadata_fields.due_date,
            discount: update_metadata_fields.discount,
            min_value_or_percentage: update_metadata_fields.min_value_or_percentage,
            max_value_or_percentage: update_metadata_fields.max_value_or_percentage,
            interest: update_metadata_fields.interest,
        })
    }
}

fn validate_metadata_fields(
    metadata: &common_utils::pii::SecretSerdeValue,
) -> Result<SantanderBoletoUpdateRequest, errors::ConnectorError> {
    let metadata_value = metadata.clone().expose();

    let metadata_map = match metadata_value.as_object() {
        Some(map) => map,
        None => {
            return Err(errors::ConnectorError::GenericError {
                error_message: "Metadata should be a key value pair".to_string(),
                error_object: metadata_value,
            });
        }
    };

    if metadata_map.len() > 10 {
        return Err(errors::ConnectorError::GenericError {
            error_message: "Metadata field limit exceeded".to_string(),
            error_object: Value::Object(metadata_map.clone()),
        });
    }

    let parsed_metadata: SantanderBoletoUpdateRequest =
        serde_json::from_value(metadata_value.clone())
            .map_err(|_| errors::ConnectorError::ParsingFailed)?;

    Ok(parsed_metadata)
}

pub fn format_emv_field(id: &str, value: &str) -> String {
    format!("{id}{:02}{value}", value.len())
}

fn format_field(id: &str, value: &str) -> String {
    format!("{}{:02}{}", id, value.len(), value)
}

pub fn generate_emv_string(
    city: &str,
    amount: &str,
    country: enums::CountryAlpha2,
    merchant_name: &str,
    transaction_id: String,
    location: String,
) -> Result<String, errors::ConnectorError> {
    // ID 00: Payload Format Indicator
    let payload_format_indicator = format_field("00", "01");

    // ID 01: Point of Initiation Method
    let point_of_initiation_method = format_field("01", "12");

    // ID 26: Merchant Account Information
    let gui = format_field("00", "br.gov.bcb.pix");
    let loc = format_field("25", &location);
    let merchant_account_information = format_field("26", &format!("{}{}", gui, loc));

    // ID 52: Merchant Category Code
    let merchant_category_code = format_field("52", "0000");

    // ID 53: Transaction Currency
    let transaction_currency = format_field("53", "986");

    // ID 54: Transaction Amount
    let transaction_amount = format_field("54", amount);

    // ID 58: Country Code
    let country_code = format_field("58", &country.to_string());

    // ID 59: Merchant Name
    let merchant_name = format_field("59", merchant_name);

    // ID 60: Merchant City
    let merchant_city = format_field("60", city); // to consume from req

    // Format subfield 05 with the actual TXID
    // This is an optional field to be sent while creating the copy-and-paste data for Pix QR Code
    // If sent, pass the first 25 or last 25 letters, if not passed then pass 3 astericks
    let reference_label = format_field("05", &transaction_id.chars().take(25).collect::<String>());

    // Wrap it inside ID 62
    let additional_data = format_field("62", &reference_label);

    let emv_without_crc = format!(
        "{}{}{}{}{}{}{}{}{}{}",
        payload_format_indicator,
        point_of_initiation_method,
        merchant_account_information,
        merchant_category_code,
        transaction_currency,
        transaction_amount,
        country_code,
        merchant_name,
        merchant_city,
        additional_data
    );
    // CRC16-CCITT-FALSE constant
    const CRC16_CCITT_FALSE: Algorithm<u16> = Algorithm {
        width: 16,
        poly: 0x1021,
        init: 0xFFFF,
        refin: false,
        refout: false,
        xorout: 0x0000,
        check: 0x29B1,
        residue: 0x0000,
    };

    // ID 63: CRC16
    let crc_payload = format!("{}6304", emv_without_crc);
    let crc_value = Crc::<u16>::new(&CRC16_CCITT_FALSE).checksum(crc_payload.as_bytes());
    let crc_hex = format!("{:04X}", crc_value);

    Ok(format!("{}{}", crc_payload, crc_hex))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SanatanderAccessTokenResponse {
    Response(SanatanderTokenResponse),
    Error(SantanderTokenErrorResponse),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SanatanderTokenResponse {
    #[serde(rename = "refreshUrl")]
    pub refresh_url: String,
    pub token_type: String,
    pub client_id: Secret<String>,
    pub access_token: Secret<String>,
    pub scopes: String,
    pub expires_in: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SantanderTokenErrorResponse {
    #[serde(rename = "type")]
    pub error_type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
}

#[derive(Default, Debug, Serialize)]
pub struct SantanderCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPSyncBoletoRequest {
    payer_document_number: Secret<i64>,
}

#[derive(Debug, Serialize)]
pub struct SantanderAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
    pub(super) certificate: Secret<String>,
    pub(super) certificate_key: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct SantanderAuthRequest {
    client_id: Secret<String>,
    client_secret: Secret<String>,
    grant_type: SantanderGrantType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SantanderGrantType {
    ClientCredentials,
}

impl TryFrom<&ConnectorAuthType> for SantanderAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Ok(Self {
                client_id: api_key.to_owned(),
                client_secret: key1.to_owned(),
                certificate: api_secret.to_owned(),
                certificate_key: key2.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl TryFrom<&RefreshTokenRouterData> for SantanderAuthRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth_details = SantanderAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            client_id: auth_details.client_id,
            client_secret: auth_details.client_secret,
            grant_type: SantanderGrantType::ClientCredentials,
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, SanatanderAccessTokenResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SanatanderAccessTokenResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            SanatanderAccessTokenResponse::Response(res) => Ok(Self {
                response: Ok(AccessToken {
                    token: res.access_token,
                    expires: res
                        .expires_in
                        .parse::<i64>()
                        .change_context(errors::ConnectorError::ParsingFailed)?,
                }),
                ..item.data
            }),
            SanatanderAccessTokenResponse::Error(error) => Ok(Self {
                response: Err(ErrorResponse {
                    code: error.error_type,
                    message: error.title,
                    reason: Some(error.detail),
                    status_code: error.status,
                    attempt_status: None,
                    connector_transaction_id: None,
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
        }
    }
}

impl TryFrom<&SantanderRouterData<&PaymentsAuthorizeRouterData>> for SantanderPaymentRequest {
    type Error = Error;
    fn try_from(
        value: &SantanderRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if value.router_data.request.capture_method != Some(enums::CaptureMethod::Automatic) {
            return Err(errors::ConnectorError::FlowNotSupported {
                flow: format!("{:?}", value.router_data.request.capture_method),
                connector: "Santander".to_string(),
            }
            .into());
        }
        match value.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankTransfer(ref bank_transfer_data) => {
                Self::try_from((value, bank_transfer_data.as_ref()))
            }
            PaymentMethodData::Voucher(ref voucher_data) => Self::try_from((value, voucher_data)),
            _ => Err(errors::ConnectorError::NotImplemented(
                crate::utils::get_unimplemented_payment_method_error_message("Santander"),
            ))?,
        }
    }
}

impl TryFrom<&SantanderRouterData<&PaymentsSyncRouterData>> for SantanderPSyncBoletoRequest {
    type Error = Error;
    fn try_from(value: &SantanderRouterData<&PaymentsSyncRouterData>) -> Result<Self, Self::Error> {
        let payer_document_number: i64 = value
            .router_data
            .connector_request_reference_id
            .parse()
            .map_err(|_| errors::ConnectorError::ParsingFailed)?;

        Ok(Self {
            payer_document_number: Secret::new(payer_document_number),
        })
    }
}

impl
    TryFrom<(
        &SantanderRouterData<&PaymentsAuthorizeRouterData>,
        &VoucherData,
    )> for SantanderPaymentRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &SantanderRouterData<&PaymentsAuthorizeRouterData>,
            &VoucherData,
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

        let nsu_code = if value
            .0
            .router_data
            .is_payment_id_from_merchant
            .unwrap_or(false)
            && value.0.router_data.payment_id.len() > 20
        {
            return Err(errors::ConnectorError::MaxFieldLengthViolated {
                connector: "Santander".to_string(),
                field_name: "payment_id".to_string(),
                max_length: 20,
                received_length: value.0.router_data.payment_id.len(),
            }
            .into());
        } else {
            value.0.router_data.payment_id.clone()
        };

        let due_date = value
            .0
            .router_data
            .request
            .feature_metadata
            .clone()
            .and_then(|fm| fm.boleto_expiry_details)
            .unwrap_or_else(|| "boleto_expiry_details".to_string());

        Ok(Self::Boleto(Box::new(SantanderBoletoPaymentRequest {
            environment: Environment::from(router_env::env::which()),
            nsu_code,
            nsu_date: time::OffsetDateTime::now_utc()
                .date()
                .format(&time::macros::format_description!("[year]-[month]-[day]"))
                .change_context(errors::ConnectorError::DateFormattingFailed)?,
            covenant_code: santander_mca_metadata.covenant_code.clone(),
            bank_number: voucher_data.bank_number.clone().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "document_type",
                }
            })?, // size: 13
            client_number: Some(value.0.router_data.get_customer_id()?),
            due_date,
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
            document_kind: BoletoDocumentKind::BillProposal, // Need confirmation
            discount: Some(Discount {
                discount_type: DiscountType::Free,
                discount_one: None,
                discount_two: None,
                discount_three: None,
            }),
            fine_percentage: value
                .0
                .router_data
                .request
                .feature_metadata
                .as_ref()
                .and_then(|fm| fm.pix_additional_details.as_ref())
                .and_then(|fine| fine.fine_percentage.clone()),
            fine_quantity_days: value
                .0
                .router_data
                .request
                .feature_metadata
                .as_ref()
                .and_then(|fm| fm.pix_additional_details.as_ref())
                .and_then(|days| days.fine_quantity_days.clone()),
            interest_percentage: value
                .0
                .router_data
                .request
                .feature_metadata
                .as_ref()
                .and_then(|fm| fm.pix_additional_details.as_ref())
                .and_then(|interest| interest.interest_percentage.clone()),
            deduction_value: None,
            protest_type: None,
            protest_quantity_days: None,
            write_off_quantity_days: value
                .0
                .router_data
                .request
                .feature_metadata
                .as_ref()
                .and_then(|fm| fm.pix_additional_details.as_ref())
                .and_then(|days| days.write_off_quantity_days.clone()),
            payment_type: PaymentType::Registration,
            parcels_quantity: None,
            value_type: None,
            min_value_or_percentage: None,
            max_value_or_percentage: None,
            iof_percentage: None,
            sharing: None,
            key: None,
            tx_id: None,
            messages: value
                .0
                .router_data
                .request
                .feature_metadata
                .as_ref()
                .and_then(|fm| fm.pix_additional_details.as_ref())
                .and_then(|messages| messages.messages.clone()),
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

        let (calendar, debtor) = match &value
            .0
            .router_data
            .request
            .feature_metadata
            .as_ref()
            .and_then(|f| f.pix_qr_expiry_time.as_ref())
        {
            Some(api_models::payments::PixQRExpirationDuration::Immediate(val)) => {
                let cal =
                    SantanderPixRequestCalendar::Immediate(SantanderPixImmediateCalendarRequest {
                        expiration: val.time,
                    });
                let debt = Some(SantanderDebtor {
                    cnpj: Some(santander_mca_metadata.cnpj.clone()),
                    name: value.0.router_data.get_billing_full_name()?,
                    street: None,
                    city: None,
                    state: None,
                    zip_code: None,
                    cpf: None,
                });

                (cal, debt)
            }
            Some(api_models::payments::PixQRExpirationDuration::Scheduled(val)) => {
                let cal =
                    SantanderPixRequestCalendar::Scheduled(SantanderPixDueDateCalendarRequest {
                        expiration_date: val.date.clone(),
                        validity_after_expiration: val.validity_after_expiration,
                    });

                let debt = Some(SantanderDebtor {
                    cpf: Some(santander_mca_metadata.cpf.clone()),
                    name: value.0.router_data.get_billing_full_name()?,
                    street: None,
                    city: None,
                    state: None,
                    zip_code: None,
                    cnpj: None,
                });

                (cal, debt)
            }
            None => {
                let cal =
                    SantanderPixRequestCalendar::Immediate(SantanderPixImmediateCalendarRequest {
                        expiration: 3600, // default 1 hour
                    });

                let debt = Some(SantanderDebtor {
                    cnpj: Some(santander_mca_metadata.cpf.clone()),
                    name: value.0.router_data.get_billing_full_name()?,
                    street: None,
                    city: None,
                    state: None,
                    zip_code: None,
                    cpf: None,
                });

                (cal, debt)
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
#[serde(untagged)]
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoPaymentRequest {
    pub environment: Environment,
    pub nsu_code: String,
    pub nsu_date: String,
    pub covenant_code: String,
    pub bank_number: Secret<String>,
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
    pub value: f64,
    pub limit_date: String, // YYYY-MM-DD
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
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    #[serde(rename = "type")]
    pub key_type: Option<String>,
    pub dict_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderDebtor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnpj: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpf: Option<Secret<String>>,
    #[serde(rename = "nome")]
    pub name: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "logradouro")]
    pub street: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cidade")]
    pub city: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "uf")]
    pub state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "cep")]
    pub zip_code: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderValue {
    pub original: StringMajorUnit,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderAdditionalInfo {
    #[serde(rename = "nome")]
    pub name: String,
    #[serde(rename = "valor")]
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SantanderPaymentStatus {
    #[serde(rename = "ATIVA")]
    Active,
    #[serde(rename = "CONCLUIDA")]
    Completed,
    #[serde(rename = "REMOVIDA_PELO_USUARIO_RECEBEDOR")]
    RemovedByReceivingUser,
    #[serde(rename = "REMOVIDA_PELO_PSP")]
    RemovedByPSP,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SantanderVoidStatus {
    #[serde(rename = "REMOVIDA_PELO_USUARIO_RECEBEDOR")]
    RemovedByReceivingUser,
}

impl From<SantanderPaymentStatus> for AttemptStatus {
    fn from(item: SantanderPaymentStatus) -> Self {
        match item {
            SantanderPaymentStatus::Active => Self::AuthenticationPending,
            SantanderPaymentStatus::Completed => Self::Charged,
            SantanderPaymentStatus::RemovedByReceivingUser => Self::Voided,
            SantanderPaymentStatus::RemovedByPSP => Self::Failure,
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
#[serde(untagged)]
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
    pub bank_number: Secret<String>,
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
    pub digitable_line: Option<Secret<String>>,
    pub entry_date: Option<String>,
    pub qr_code_pix: Option<String>,
    pub qr_code_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderPixRequestCalendar {
    Immediate(SantanderPixImmediateCalendarRequest),
    Scheduled(SantanderPixDueDateCalendarRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixDueDateCalendarRequest {
    #[serde(rename = "dataDeVencimento")]
    pub expiration_date: String,
    #[serde(rename = "validadeAposVencimento")]
    pub validity_after_expiration: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixQRCodePaymentsResponse {
    pub status: SantanderPaymentStatus,
    #[serde(rename = "calendario")]
    pub calendar: SantanderCalendarResponse,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: Option<Value>,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    pub location: Option<String>,
    #[serde(rename = "recebedor")]
    pub recipient: Recipient,
    #[serde(rename = "valor")]
    pub value: SantanderValue,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
    pub pix: Option<Vec<SantanderPix>>,
    #[serde(rename = "pixCopiaECola")]
    pub pix_qr_code_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixQRCodeSyncResponse {
    pub status: SantanderPaymentStatus,
    #[serde(rename = "calendario")]
    pub calendar: SantanderCalendarResponse,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: Value,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    pub location: Option<String>,
    #[serde(rename = "valor")]
    pub value: SantanderValue,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
    pub pix: Option<Vec<SantanderPix>>,
    #[serde(rename = "pixCopiaECola")]
    pub pix_qr_code_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderPixQRPaymentRequest {
    #[serde(rename = "calendario")]
    pub calendar: SantanderPixRequestCalendar,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    #[serde(rename = "valor")]
    pub value: SantanderValue,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(rename = "infoAdicionais")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixVoidResponse {
    #[serde(rename = "calendario")]
    pub calendar: SantanderCalendarResponse,
    #[serde(rename = "txid")]
    pub transaction_id: String,
    #[serde(rename = "revisao")]
    pub revision: Value,
    #[serde(rename = "devedor")]
    pub debtor: Option<SantanderDebtor>,
    #[serde(rename = "recebedor")]
    pub recebedor: Recipient,
    pub status: SantanderPaymentStatus,
    #[serde(rename = "valor")]
    pub value: ValueResponse,
    #[serde(rename = "pixCopiaECola")]
    pub pix_qr_code_data: Option<Secret<String>>,
    #[serde(rename = "chave")]
    pub key: Secret<String>,
    #[serde(rename = "solicitacaoPagador")]
    pub request_payer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "infoAdicionais")]
    pub additional_info: Option<Vec<SantanderAdditionalInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueResponse {
    #[serde(rename = "original")]
    pub original: String,
    #[serde(rename = "multa")]
    pub fine: Fine,
    #[serde(rename = "juros")]
    pub interest: Interest,
    #[serde(rename = "desconto")]
    pub discount: DiscountResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fine {
    #[serde(rename = "modalidade")]
    pub r#type: String,
    #[serde(rename = "valorPerc")]
    pub perc_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interest {
    #[serde(rename = "modalidade")]
    pub r#type: String,
    #[serde(rename = "valorPerc")]
    pub perc_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountResponse {
    #[serde(rename = "modalidade")]
    pub r#type: String,
    #[serde(rename = "descontoDataFixa")]
    pub fixed_date_discount: Vec<FixedDateDiscount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedDateDiscount {
    #[serde(rename = "data")]
    pub date: String,
    #[serde(rename = "valorPerc")]
    pub perc_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    pub cnpj: Option<Secret<String>>,
    #[serde(rename = "nome")]
    pub name: Option<Secret<String>>,
    #[serde(rename = "nomeFantasia")]
    pub business_name: Option<Secret<String>>,
    #[serde(rename = "logradouro")]
    pub street: Option<Secret<String>>,
    #[serde(rename = "cidade")]
    pub city: Option<Secret<String>>,
    #[serde(rename = "uf")]
    pub state: Option<Secret<String>>,
    #[serde(rename = "cep")]
    pub zip_code: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderPixImmediateCalendarRequest {
    #[serde(rename = "expiracao")]
    pub expiration: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SantanderCalendarResponse {
    #[serde(rename = "criacao")]
    pub creation: String,
    #[serde(rename = "expiracao")]
    pub expiration: Option<String>,
    #[serde(rename = "dataDeVencimento")]
    pub due_date: Option<String>,
    #[serde(rename = "validadeAposVencimento")]
    pub validity_after_due: Option<i64>,        // changed this from String to i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SantanderPaymentsSyncResponse {
    PixQRCode(Box<SantanderPixQRCodeSyncResponse>),
    Boleto(Box<SantanderBoletoPSyncResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SantanderBoletoPSyncResponse {
    pub link: Option<Url>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderPixCancelRequest {
    pub status: Option<SantanderVoidStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SantanderPaymentsCancelRequest {
    PixQR(SantanderPixCancelRequest),
    Boleto(SantanderBoletoCancelRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderBoletoCancelRequest {
    pub covenant_code: String,
    pub bank_number: String,
    pub operation: SantanderBoletoCancelOperation,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum SantanderBoletoCancelOperation {
    #[serde(rename = "PROTESTAR")]
    Protest,
    #[serde(rename = "CANCELAR_PROTESTO")]
    CancelProtest,
    #[serde(rename = "BAIXAR")]
    #[default]
    WriteOff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SantanderUpdateBoletoResponse {
    pub covenant_code: Option<String>,
    pub bank_number: Option<String>,
    pub message: Option<String>,
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
                let attempt_status = AttemptStatus::from(pix_data.status.clone());
                match attempt_status {
                    AttemptStatus::Failure => {
                        let response = Err(get_sync_error_response(
                            Box::new(*pix_data),
                            item.http_code,
                            attempt_status,
                        ));
                        Ok(Self {
                            response,
                            ..item.data
                        })
                    }
                    _ => {
                        let connector_metadata = pix_data
                            .pix
                            .ok_or_else(|| errors::ConnectorError::ParsingFailed)?
                            .first()
                            .map(|pix| {
                                serde_json::json!({
                                    "end_to_end_id": pix.end_to_end_id.clone().expose()
                                })
                            });
                        Ok(Self {
                            status: AttemptStatus::from(pix_data.status),
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
                }
            }
            SantanderPaymentsSyncResponse::Boleto(boleto_data) => {
                let redirection_data = boleto_data.link.clone().map(|url| RedirectForm::Form {
                    endpoint: url.to_string(),
                    method: Method::Get,
                    form_fields: HashMap::new(),
                });

                Ok(Self {
                    status: AttemptStatus::AuthenticationPending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::NoResponseId,
                        redirection_data: Box::new(redirection_data),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
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

pub fn get_error_response(
    pix_data: Box<SantanderPixQRCodePaymentsResponse>,
    status_code: u16,
    attempt_status: AttemptStatus,
) -> ErrorResponse {
    ErrorResponse {
        code: NO_ERROR_CODE.to_string(),
        message: NO_ERROR_MESSAGE.to_string(),
        reason: None,
        status_code,
        attempt_status: Some(attempt_status),
        connector_transaction_id: Some(pix_data.transaction_id.clone()),
        network_advice_code: None,
        network_decline_code: None,
        network_error_message: None,
        connector_metadata: None,
    }
}

pub fn get_sync_error_response(
    pix_data: Box<SantanderPixQRCodeSyncResponse>,
    status_code: u16,
    attempt_status: AttemptStatus,
) -> ErrorResponse {
    ErrorResponse {
        code: NO_ERROR_CODE.to_string(),
        message: NO_ERROR_MESSAGE.to_string(),
        reason: None,
        status_code,
        attempt_status: Some(attempt_status),
        connector_transaction_id: Some(pix_data.transaction_id.clone()),
        network_advice_code: None,
        network_decline_code: None,
        network_error_message: None,
        connector_metadata: None,
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
            SantanderPaymentsResponse::PixQRCode(pix_data) => {
                let attempt_status = AttemptStatus::from(pix_data.status.clone());
                match attempt_status {
                    AttemptStatus::Failure => {
                        let response = Err(get_error_response(
                            Box::new(*pix_data),
                            item.http_code,
                            attempt_status,
                        ));
                        Ok(Self {
                            response,
                            ..item.data
                        })
                    }
                    _ => Ok(Self {
                        status: AttemptStatus::from(pix_data.status.clone()),
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
                }
            }
            SantanderPaymentsResponse::Boleto(boleto_data) => {
                let voucher_data = VoucherNextStepData {
                    expires_at: None,
                    digitable_line: boleto_data.digitable_line.clone(),
                    reference: boleto_data.barcode.ok_or(
                        errors::ConnectorError::MissingConnectorRedirectionPayload {
                            field_name: "barcode",
                        },
                    )?,
                    entry_date: boleto_data.entry_date,
                    download_url: None,
                    instructions_url: None,
                    bank_number: Some(boleto_data.bank_number.clone()),
                };

                let connector_metadata = Some(voucher_data.encode_to_value())
                    .transpose()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

                let resource_id = match boleto_data.tx_id {
                    Some(tx_id) => ResponseId::ConnectorTransactionId(tx_id),
                    None => ResponseId::NoResponseId,
                };

                let connector_response_reference_id = Some(
                    boleto_data
                        .digitable_line
                        .clone()
                        .map(|data| data.expose())
                        .or_else(|| {
                            boleto_data.beneficiary.as_ref().map(|beneficiary| {
                                format!(
                                    "{:?}.{:?}",
                                    boleto_data.bank_number,
                                    beneficiary.document_number.clone()
                                )
                            })
                        })
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "beneficiary.document_number",
                        })?,
                );

                Ok(Self {
                    status: AttemptStatus::AuthenticationPending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id,
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata,
                        network_txn_id: None,
                        connector_response_reference_id,
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, SantanderPixVoidResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SantanderPixVoidResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item.response.clone();
        Ok(Self {
            status: AttemptStatus::from(item.response.status),
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
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let santander_mca_metadata = SantanderMetadataObject::try_from(&item.connector_meta_data)?;

        match item.payment_method {
            enums::PaymentMethod::BankTransfer => match item.request.payment_method_type {
                Some(enums::PaymentMethodType::Pix) => Ok(Self::PixQR(SantanderPixCancelRequest {
                    status: Some(SantanderVoidStatus::RemovedByReceivingUser),
                })),
                _ => Err(errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_method",
                }
                .into()),
            },
            enums::PaymentMethod::Voucher => match item.request.payment_method_type {
                Some(enums::PaymentMethodType::Boleto) => {
                    Ok(Self::Boleto(SantanderBoletoCancelRequest {
                        operation: SantanderBoletoCancelOperation::WriteOff,
                        covenant_code: santander_mca_metadata.covenant_code.clone(),
                        bank_number: extract_bank_number(item.request.connector_meta.clone())?,
                    }))
                }
                _ => Err(errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_method",
                }
                .into()),
            },
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method",
            }
            .into()),
        }
    }
}

fn extract_bank_number(value: Option<Value>) -> Result<String, errors::ConnectorError> {
    let value = value.ok_or_else(|| errors::ConnectorError::NoConnectorMetaData)?;

    let map = value
        .as_object()
        .ok_or_else(|| errors::ConnectorError::NoConnectorMetaData)?;

    let bank_number = map
        .get("bank_number")
        .ok_or_else(|| errors::ConnectorError::NoConnectorMetaData)?;

    let bank_number_str = bank_number
        .as_str()
        .ok_or_else(|| errors::ConnectorError::NoConnectorMetaData)?
        .to_string();

    Ok(bank_number_str)
}

fn get_qr_code_data<F, T>(
    item: &ResponseRouterData<F, SantanderPaymentsResponse, T, PaymentsResponseData>,
    pix_data: &SantanderPixQRCodePaymentsResponse,
) -> CustomResult<Option<Value>, errors::ConnectorError> {
    // Scheduled type Pix QR Code Response already has a formed emv string data for QR Code
    // HS doesnt need to create it
    if let Some(data) = pix_data.pix_qr_code_data.clone() {
        return convert_pix_data_to_value(
            data,
            Some(api_models::payments::SantanderVariant::Scheduled),
        );
    }

    let santander_mca_metadata = SantanderMetadataObject::try_from(&item.data.connector_meta_data)?;

    let response = pix_data.clone();

    let merchant_city = santander_mca_metadata.merchant_city.as_str();

    let merchant_name = santander_mca_metadata.merchant_name.as_str();

    let amount_i64 = StringMajorUnitForConnector
        .convert_back(response.value.original, enums::Currency::BRL)
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?
        .get_amount_as_i64();

    let amount_string = amount_i64.to_string();
    let amount = amount_string.as_str();

    let location = pix_data
        .location
        .clone()
        .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

    let dynamic_pix_code = generate_emv_string(
        merchant_city,
        amount,
        item.data.get_billing_country()?,
        merchant_name,
        pix_data.transaction_id.clone(),
        location,
    )?;

    let variant = if pix_data.pix_qr_code_data.is_some() {
        Some(api_models::payments::SantanderVariant::Scheduled)
    } else {
        Some(api_models::payments::SantanderVariant::Immediate)
    };

    return convert_pix_data_to_value(dynamic_pix_code, variant);
}

fn convert_pix_data_to_value(
    data: String,
    variant: Option<api_models::payments::SantanderVariant>,
) -> CustomResult<Option<Value>, errors::ConnectorError> {
    let image_data = QrImage::new_from_data(data.clone())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let image_data_url = Url::parse(image_data.data.clone().as_str())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let qr_code_info = QrCodeInformation::QrDataUrlSantander {
        qr_code_url: image_data_url,
        display_to_timestamp: None,
        variant,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct QrDataUrlSantander {
    pub qr_code_url: Url,
    pub display_to_timestamp: Option<i64>,
    pub variant: Option<api_models::payments::SantanderVariant>,
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
#[serde(untagged)]
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
    pub status: String,
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
    pub final_beneficiary_document_type: enums::DocumentKind,
    pub final_beneficiary_document_number: String,
    pub final_beneficiary_name: String,
    pub due_date: String,
    pub nominal_value: StringMajorUnit,
    #[serde(rename = "payed_value")]
    pub paid_value: String,
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
    #[serde(rename = "AgenciasAutoAtendimento")]
    BankBranchOrAtm,

    /// Payment made through online banking.
    #[serde(rename = "InternetBanking")]
    OnlineBanking,

    /// Payment made at a physical correspondent agent (e.g., convenience stores, partner outlets).
    #[serde(rename = "CorrespondenteBancarioFisico")]
    PhysicalCorrespondentAgent,

    /// Payment made via Santander’s call center.
    #[serde(rename = "CentralDeAtendimento")]
    CallCenter,

    /// Payment made via electronic file, typically for bulk company payments.
    #[serde(rename = "ArquivoEletronico")]
    ElectronicFile,

    /// Payment made via DDA (Débito Direto Autorizado) / electronic bill presentment system.
    #[serde(rename = "Dda")]
    DirectDebitAuthorized,

    /// Payment made via digital correspondent channels (apps, kiosks, digital partners).
    #[serde(rename = "CorrespondenteBancarioDigital")]
    DigitalCorrespondentAgent,
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
