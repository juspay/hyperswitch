use api_models::payments::{ExpiryType, QrCodeInformation, VoucherNextStepData};
use common_enums::{enums, AttemptStatus, BoletoDocumentKind, BoletoPaymentType};
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, Encode, ValueExt},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use crc::{Algorithm, Crc};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, BoletoVoucherData, PaymentMethodData, VoucherData},
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_request_types::{PaymentsUpdateMetadataData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
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
    connectors::santander::{
        requests::{
            Environment, SantanderAuthRequest, SantanderAuthType, SantanderBoletoCancelOperation,
            SantanderBoletoCancelRequest, SantanderBoletoPaymentRequest,
            SantanderBoletoUpdateRequest, SantanderDebtor, SantanderGrantType,
            SantanderMetadataObject, SantanderPaymentRequest, SantanderPaymentsCancelRequest,
            SantanderPixCancelRequest, SantanderPixDueDateCalendarRequest,
            SantanderPixImmediateCalendarRequest, SantanderPixQRPaymentRequest,
            SantanderPixRequestCalendar, SantanderRefundRequest, SantanderRouterData,
            SantanderValue,
        },
        responses::{
            FunctionType, NsuComposite, Payer, SanatanderAccessTokenResponse,
            SanatanderTokenResponse, SantanderBoletoDocumentKind, SantanderBoletoPaymentType,
            SantanderPaymentStatus, SantanderPaymentsResponse, SantanderPaymentsSyncResponse,
            SantanderPixQRCodePaymentsResponse, SantanderPixQRCodeSyncResponse,
            SantanderRefundResponse, SantanderRefundStatus, SantanderUpdateMetadataResponse,
            SantanderVoidResponse, SantanderVoidStatus, SantanderWebhookBody,
        },
    },
    types::{RefreshTokenRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{self as connector_utils, QrImage, RouterData as _},
};

type Error = error_stack::Report<errors::ConnectorError>;

impl<T> From<(StringMajorUnit, T)> for SantanderRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
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
        let santander_mca_metadata = SantanderMetadataObject::try_from(&item.connector_meta_data)?;

        let boleto_mca_metadata = santander_mca_metadata
            .boleto
            .ok_or(errors::ConnectorError::NoConnectorMetaData)?;

        let boleto_components = extract_boleto_components(&item.request.connector_transaction_id)?;

        Ok(Self {
            covenant_code: boleto_mca_metadata.covenant_code,
            bank_number: boleto_components.bank_number,
            due_date: item
                .request
                .feature_metadata
                .clone()
                .and_then(|data| data.boleto_additional_details)
                .and_then(|boleto_details| boleto_details.due_date),
        })
    }
}

pub fn format_emv_field(id: &str, value: &str) -> String {
    format!("{id}{:02}{value}", value.len())
}

fn format_field(id: &str, value: &str) -> String {
    format!("{}{:02}{}", id, value.len(), value)
}

pub fn generate_emv_string(
    cidade: &str,
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
    let merchant_city = format_field("60", cidade); // to consume from req

    // Format subfield 05 with the actual TXID
    // This is an optional field to be sent while creating the copy-and-paste data for Pix QR Code
    // If sent, pass the first 25 or last 25 letters, if not passed then pass 3 astericks
    let reference_label = format_field("05", &transaction_id.chars().take(25).collect::<String>());

    // Wrap it inside ID 62
    let additional_data = format_field("62", &reference_label);

    let emv_without_crc = format!(
        "{payload_format_indicator}{point_of_initiation_method}{merchant_account_information}{merchant_category_code}{transaction_currency}{transaction_amount}{country_code}{merchant_name}{merchant_city}{additional_data}",
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
#[serde(rename_all = "camelCase")]
pub struct SantanderPSyncBoletoRequest {
    payer_document_number: Secret<i64>,
}

impl TryFrom<&ConnectorAuthType> for SantanderAuthType {
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

impl TryFrom<(&RefreshTokenRouterData, &SantanderMetadataObject)> for SantanderAuthRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: (&RefreshTokenRouterData, &SantanderMetadataObject),
    ) -> Result<Self, Self::Error> {
        let (client_id, client_secret) = match item.0.payment_method_type {
            Some(enums::PaymentMethodType::Pix) => {
                let pix_mca_metadata = item
                    .1
                    .pix
                    .as_ref()
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)?;
                Ok((
                    pix_mca_metadata.client_id.clone(),
                    pix_mca_metadata.client_secret.clone(),
                ))
            }
            Some(enums::PaymentMethodType::Boleto) => {
                let boleto_mca_metadata = item
                    .1
                    .boleto
                    .as_ref()
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)?;
                Ok((
                    boleto_mca_metadata.client_id.clone(),
                    boleto_mca_metadata.client_secret.clone(),
                ))
            }
            _ => Err(error_stack::report!(errors::ConnectorError::NotSupported {
                message: item.0.payment_method.to_string(),
                connector: "Santander",
            })),
        }?;

        Ok(Self {
            client_id,
            client_secret,
            grant_type: SantanderGrantType::ClientCredentials,
        })
    }
}

impl TryFrom<&ConnectorAuthType> for SantanderAuthRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(connector_auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        let auth_details = SantanderAuthType::try_from(connector_auth_type)?;

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
            SanatanderAccessTokenResponse::Response(res) => match res {
                SanatanderTokenResponse::Pix(pix_response) => Ok(Self {
                    response: Ok(AccessToken {
                        token: pix_response.access_token,
                        expires: pix_response
                            .expires_in
                            .parse::<i64>()
                            .change_context(errors::ConnectorError::ParsingFailed)?,
                    }),
                    ..item.data
                }),
                SanatanderTokenResponse::Boleto(boleto_response) => Ok(Self {
                    response: Ok(AccessToken {
                        token: boleto_response.access_token,
                        expires: boleto_response.expires_in,
                    }),
                    ..item.data
                }),
            },
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
                    connector_response_reference_id: None,
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
            PaymentMethodData::Voucher(ref voucher_data) => match voucher_data {
                VoucherData::Boleto(boleto_data) => Self::try_from((value, boleto_data.as_ref())),
                _ => Err(errors::ConnectorError::NotImplemented(
                    crate::utils::get_unimplemented_payment_method_error_message("Santander"),
                )
                .into()),
            },
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
        &BoletoVoucherData,
    )> for SantanderPaymentRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &SantanderRouterData<&PaymentsAuthorizeRouterData>,
            &BoletoVoucherData,
        ),
    ) -> Result<Self, Self::Error> {
        let santander_mca_metadata =
            SantanderMetadataObject::try_from(&value.0.router_data.connector_meta_data)?;

        let boleto_mca_metadata = santander_mca_metadata
            .boleto
            .ok_or(errors::ConnectorError::NoConnectorMetaData)?;

        let voucher_data = value.1;

        let nsu_code = Some(value.0.router_data.connector_request_reference_id.clone());

        let bank_number = nsu_code.clone();

        let (due_date, payment_type, document_kind) = {
            let details = value
                .0
                .router_data
                .request
                .feature_metadata
                .as_ref()
                .and_then(|fm| fm.boleto_additional_details.as_ref())
                .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                    field_name: "feature_metadata.boleto_additional_details",
                })?;

            (
                Some(details.due_date.as_ref().cloned().ok_or_else(|| {
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "feature_metadata.boleto_additional_details.due_date",
                    }
                })?),
                Some(details.payment_type.as_ref().copied().ok_or_else(|| {
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "feature_metadata.boleto_additional_details.payment_type",
                    }
                })?),
                Some(details.document_kind.as_ref().copied().ok_or_else(|| {
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "feature_metadata.boleto_additional_details.document_kind",
                    }
                })?),
            )
        };

        Ok(Self::Boleto(Box::new(SantanderBoletoPaymentRequest {
            environment: Some(Environment::Producao),
            nsu_code,
            nsu_date: Some(
                time::OffsetDateTime::now_utc()
                    .date()
                    .format(&time::macros::format_description!("[year]-[month]-[day]"))
                    .change_context(errors::ConnectorError::DateFormattingFailed)?,
            ),
            covenant_code: Some(boleto_mca_metadata.covenant_code.clone()),
            bank_number,
            client_number: value
                .0
                .router_data
                .request
                .merchant_order_reference_id
                .clone(),
            due_date,
            issue_date: Some(
                time::OffsetDateTime::now_utc()
                    .date()
                    .format(&time::macros::format_description!("[year]-[month]-[day]"))
                    .change_context(errors::ConnectorError::DateFormattingFailed)?,
            ),
            nominal_value: Some(value.0.amount.to_owned()),
            participant_code: value
                .0
                .router_data
                .request
                .merchant_order_reference_id
                .clone(),
            payer: Some(Payer {
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
                city: Secret::new(value.0.router_data.get_billing_city()?),
                state: value.0.router_data.get_billing_state()?,
                zip_code: value.0.router_data.get_billing_zip()?,
            }),
            beneficiary: None,
            document_kind: document_kind.map(SantanderBoletoDocumentKind::from),
            discount: None,
            fine_percentage: None,
            fine_quantity_days: None,
            interest_percentage: None,
            deduction_value: None,
            protest_type: None,
            protest_quantity_days: None,
            write_off_quantity_days: None,
            payment_type: payment_type.map(SantanderBoletoPaymentType::from),
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
                .billing_descriptor
                .clone()
                .and_then(|data| data.statement_descriptor)
                .map(|s| vec![s]),
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

        let pix_mca_metadata = santander_mca_metadata
            .pix
            .ok_or(errors::ConnectorError::NoConnectorMetaData)?;

        let pix_data = match value.1 {
            BankTransferData::Pix {
                pix_key,
                cpf,
                cnpj,
                source_bank_account_id,
                destination_bank_account_id,
                expiry_date,
            } => (
                pix_key,
                cpf,
                cnpj,
                source_bank_account_id,
                destination_bank_account_id,
                expiry_date,
            ),
            _ => Err(errors::ConnectorError::NotImplemented(
                crate::utils::get_unimplemented_payment_method_error_message("Santander"),
            ))?,
        };

        let (calendar, debtor) = match &value
            .0
            .router_data
            .request
            .feature_metadata
            .as_ref()
            .and_then(|f| f.pix_additional_details.as_ref())
        {
            Some(api_models::payments::PixAdditionalDetails::Immediate(val)) => {
                let cal =
                    SantanderPixRequestCalendar::Immediate(SantanderPixImmediateCalendarRequest {
                        expiracao: val.time,
                    });
                let debt = Some(SantanderDebtor {
                    cnpj: pix_data.2.clone(),
                    nome: value.0.router_data.get_billing_full_name()?,
                    logradouro: None,
                    cidade: None,
                    uf: None,
                    cep: None,
                    cpf: pix_data.1.clone(),
                });

                (Some(cal), debt)
            }
            Some(api_models::payments::PixAdditionalDetails::Scheduled(val)) => {
                let cal =
                    SantanderPixRequestCalendar::Scheduled(SantanderPixDueDateCalendarRequest {
                        data_de_vencimento: val.date.clone(),
                        validade_apos_vencimento: val.validity_after_expiration,
                    });

                let debt = Some(SantanderDebtor {
                    cpf: pix_data.1.clone(),
                    nome: value.0.router_data.get_billing_full_name()?,
                    logradouro: None,
                    cidade: None,
                    uf: None,
                    cep: None,
                    cnpj: pix_data.2.clone(),
                });

                (Some(cal), debt)
            }
            None => {
                let cal =
                    SantanderPixRequestCalendar::Immediate(SantanderPixImmediateCalendarRequest {
                        expiracao: 3600, // default 1 hour
                    });

                let debt = Some(SantanderDebtor {
                    cnpj: pix_data.2.clone(),
                    nome: value.0.router_data.get_billing_full_name()?,
                    logradouro: None,
                    cidade: None,
                    uf: None,
                    cep: None,
                    cpf: pix_data.1.clone(),
                });

                (Some(cal), debt)
            }
        };

        Ok(Self::PixQR(Box::new(SantanderPixQRPaymentRequest {
            calendario: calendar,
            devedor: debtor,
            valor: Some(SantanderValue {
                original: value.0.amount.to_owned(),
            }),
            chave: Some(pix_mca_metadata.pix_key.clone()),
            solicitacao_pagador: value
                .0
                .router_data
                .request
                .billing_descriptor
                .clone()
                .and_then(|data| data.statement_descriptor),
            info_adicionais: None,
        })))
    }
}

impl From<SantanderPaymentStatus> for AttemptStatus {
    fn from(item: SantanderPaymentStatus) -> Self {
        match item {
            SantanderPaymentStatus::Ativa => Self::AuthenticationPending,
            SantanderPaymentStatus::Concluida => Self::Charged,
            SantanderPaymentStatus::RemovidaPeloUsuarioRecebedor => Self::Voided,
            SantanderPaymentStatus::RemovidaPeloPsp => Self::Failure,
        }
    }
}

impl From<BoletoDocumentKind> for SantanderBoletoDocumentKind {
    fn from(item: BoletoDocumentKind) -> Self {
        match item {
            BoletoDocumentKind::CommercialInvoice => Self::DuplicataMercantil,
            BoletoDocumentKind::ServiceInvoice => Self::DuplicataServico,
            BoletoDocumentKind::PromissoryNote => Self::NotaPromissoria,
            BoletoDocumentKind::RuralPromissoryNote => Self::NotaPromissoriaRural,
            BoletoDocumentKind::Receipt => Self::Recibo,
            BoletoDocumentKind::InsurancePolicy => Self::ApoliceSeguro,
            BoletoDocumentKind::CreditCardInvoice => Self::BoletoCartaoCredito,
            BoletoDocumentKind::Proposal => Self::BoletoProposta,
            BoletoDocumentKind::DepositOrFunding => Self::BoletoDepositoAporte,
            BoletoDocumentKind::Cheque => Self::Cheque,
            BoletoDocumentKind::DirectPromissoryNote => Self::NotaPromissoriaDireta,
            BoletoDocumentKind::Other => Self::Outros,
        }
    }
}

impl From<BoletoPaymentType> for SantanderBoletoPaymentType {
    fn from(item: BoletoPaymentType) -> Self {
        match item {
            BoletoPaymentType::FixedAmount => Self::Registro,
            BoletoPaymentType::FlexibleAmount => Self::Divergente,
            BoletoPaymentType::Installment => Self::Parcial,
        }
    }
}

impl From<router_env::env::Env> for Environment {
    fn from(item: router_env::env::Env) -> Self {
        match item {
            router_env::env::Env::Sandbox
            | router_env::env::Env::Development
            | router_env::env::Env::Integ => Self::Teste,
            router_env::env::Env::Production => Self::Producao,
        }
    }
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
                                    pix_data.txid.clone(),
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
            SantanderPaymentsSyncResponse::Boleto(_) => Ok(Self {
                response: item.data.response,
                ..item.data
            }),
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
        connector_transaction_id: Some(pix_data.txid.clone()),
        network_advice_code: None,
        network_decline_code: None,
        network_error_message: None,
        connector_metadata: None,
        connector_response_reference_id: None,
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
        connector_transaction_id: Some(pix_data.txid.clone()),
        connector_response_reference_id: None,
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
                            resource_id: ResponseId::ConnectorTransactionId(pix_data.txid.clone()),
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
                    digitable_line: boleto_data.digitable_line.clone(),
                    expires_at: None, // have to convert a date to seconds in i64
                    reference: boleto_data.nsu_code.clone(),
                    entry_date: boleto_data.entry_date.clone(),
                    download_url: None,
                    instructions_url: None,
                };

                let connector_metadata = Some(voucher_data.encode_to_value())
                    .transpose()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

                let bank_slip_id = format!(
                    "{}.{}.P.{}.{}",
                    boleto_data.nsu_code.clone(),
                    boleto_data.nsu_date.clone(),
                    boleto_data.covenant_code.clone().expose(),
                    boleto_data.bank_number.clone(),
                );

                Ok(Self {
                    status: AttemptStatus::AuthenticationPending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(bank_slip_id),
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

use crate::connectors::santander::SantanderVoidResponse::{Boleto, Pix};

impl<F, T> TryFrom<ResponseRouterData<F, SantanderVoidResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, SantanderVoidResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.clone() {
            Pix(res) => Ok(Self {
                status: AttemptStatus::from(res.status),
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(res.txid.clone()),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            Boleto(_) => Ok(Self {
                status: AttemptStatus::Voided,
                response: item.data.response,
                ..item.data
            }),
        }
    }
}

impl TryFrom<&PaymentsCancelRouterData> for SantanderPaymentsCancelRequest {
    type Error = Error;

    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let santander_mca_metadata = SantanderMetadataObject::try_from(&item.connector_meta_data)?;

        match item.payment_method {
            enums::PaymentMethod::BankTransfer => {
                let pix_req = SantanderPixCancelRequest::try_from(item)?;
                Ok(Self::PixQR(pix_req))
            }
            enums::PaymentMethod::Voucher => {
                let boleto_req =
                    SantanderBoletoCancelRequest::try_from((item, santander_mca_metadata))?;
                Ok(Self::Boleto(boleto_req))
            }
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method",
            }
            .into()),
        }
    }
}

impl TryFrom<(&PaymentsCancelRouterData, SantanderMetadataObject)>
    for SantanderBoletoCancelRequest
{
    type Error = Error;
    fn try_from(
        value: (&PaymentsCancelRouterData, SantanderMetadataObject),
    ) -> Result<Self, Self::Error> {
        let boleto_mca_metadata = value
            .1
            .boleto
            .ok_or(errors::ConnectorError::NoConnectorMetaData)?;
        Ok(Self {
            operation: SantanderBoletoCancelOperation::Baixar,
            covenant_code: boleto_mca_metadata.covenant_code.clone(),
            bank_number: extract_bank_number(value.0.request.connector_meta.clone())?,
        })
    }
}

impl TryFrom<&PaymentsCancelRouterData> for SantanderPixCancelRequest {
    type Error = Error;
    fn try_from(_value: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            status: Some(SantanderVoidStatus::RemovidaPeloUsuarioRecebedor),
        })
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
    if let Some(data) = pix_data.pix_copia_e_cola.clone() {
        return convert_pix_data_to_value(data, Some(ExpiryType::Scheduled));
    }

    let santander_mca_metadata = SantanderMetadataObject::try_from(&item.data.connector_meta_data)?;

    let pix_mca_metadata = santander_mca_metadata
        .pix
        .ok_or(errors::ConnectorError::NoConnectorMetaData)?;

    let response = pix_data.clone();

    let merchant_city = pix_mca_metadata.merchant_city.as_str();

    let merchant_name = pix_mca_metadata.merchant_name.as_str();

    let amount_i64 = StringMajorUnitForConnector
        .convert_back(response.valor.original, enums::Currency::BRL)
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
        pix_data.txid.clone(),
        location,
    )?;

    let variant = if pix_data.pix_copia_e_cola.is_some() {
        Some(ExpiryType::Scheduled)
    } else {
        Some(ExpiryType::Immediate)
    };

    convert_pix_data_to_value(dynamic_pix_code, variant)
}

fn convert_pix_data_to_value(
    data: String,
    variant: Option<ExpiryType>,
) -> CustomResult<Option<Value>, errors::ConnectorError> {
    let image_data = QrImage::new_from_data(data.clone())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let image_data_url = Url::parse(image_data.data.clone().as_str())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let qr_code_info = QrCodeInformation::QrCodeImageUrl {
        qr_code_url: image_data_url,
        display_to_timestamp: None,
        expiry_type: variant,
    };

    Some(qr_code_info.encode_to_value())
        .transpose()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

impl<F> TryFrom<&SantanderRouterData<&RefundsRouterData<F>>> for SantanderRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SantanderRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            valor: item.amount.to_owned(),
        })
    }
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

impl TryFrom<&SantanderRouterData<&PaymentsUpdateMetadataRouterData>> for SantanderPaymentRequest {
    type Error = Error;
    fn try_from(
        value: &SantanderRouterData<&PaymentsUpdateMetadataRouterData>,
    ) -> Result<Self, Self::Error> {
        match value.router_data.request.payment_method_type {
            Some(common_enums::PaymentMethodType::Pix) => {
                SantanderPixQRPaymentRequest::try_from(value)
                    .map(|pix_qr| Self::PixQR(Box::new(pix_qr)))
            }
            Some(common_enums::PaymentMethodType::Boleto) => {
                SantanderBoletoPaymentRequest::try_from(value)
                    .map(|boleto| Self::Boleto(Box::new(boleto)))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                crate::utils::get_unimplemented_payment_method_error_message("Santander"),
            ))?,
        }
    }
}

impl TryFrom<&SantanderRouterData<&PaymentsUpdateMetadataRouterData>>
    for SantanderBoletoPaymentRequest
{
    type Error = Error;

    fn try_from(
        value: &SantanderRouterData<&PaymentsUpdateMetadataRouterData>,
    ) -> Result<Self, Self::Error> {
        let santander_mca_metadata =
            SantanderMetadataObject::try_from(&value.router_data.connector_meta_data)?;

        let boleto_mca_metadata = santander_mca_metadata
            .boleto
            .ok_or(errors::ConnectorError::NoConnectorMetaData)?;

        let due_date = Some(
            value
                .router_data
                .request
                .feature_metadata
                .as_ref()
                .and_then(|fm| fm.boleto_additional_details.as_ref())
                .and_then(|details| details.due_date.clone())
                .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                    field_name: "feature_metadata.boleto_additional_details.due_date",
                })?,
        );

        let boleto_components =
            extract_boleto_components(&value.router_data.request.connector_transaction_id)?;

        Ok(Self {
            bank_number: Some(boleto_components.bank_number),
            covenant_code: Some(boleto_mca_metadata.covenant_code.clone()),
            environment: None,
            due_date,
            nsu_code: None,
            nsu_date: None,
            client_number: None,
            issue_date: None,
            nominal_value: None,
            participant_code: None,
            payer: None,
            beneficiary: None,
            document_kind: None,
            discount: None,
            fine_percentage: None,
            fine_quantity_days: None,
            interest_percentage: None,
            deduction_value: None,
            protest_type: None,
            protest_quantity_days: None,
            write_off_quantity_days: None,
            payment_type: None,
            parcels_quantity: None,
            value_type: None,
            min_value_or_percentage: None,
            max_value_or_percentage: None,
            iof_percentage: None,
            sharing: None,
            key: None,
            tx_id: None,
            messages: None,
        })
    }
}

impl TryFrom<&SantanderRouterData<&PaymentsUpdateMetadataRouterData>>
    for SantanderPixQRPaymentRequest
{
    type Error = Error;

    fn try_from(
        value: &SantanderRouterData<&PaymentsUpdateMetadataRouterData>,
    ) -> Result<Self, Self::Error> {
        match value.router_data.request.payment_method_type {
            Some(common_enums::PaymentMethodType::Pix) => {
                let calendar = match &value
                    .router_data
                    .request
                    .feature_metadata
                    .as_ref()
                    .and_then(|f| f.pix_additional_details.as_ref())
                {
                    Some(api_models::payments::PixAdditionalDetails::Immediate(val)) => {
                        let cal = SantanderPixRequestCalendar::Immediate(
                            SantanderPixImmediateCalendarRequest {
                                expiracao: val.time,
                            },
                        );
                        Some(cal)
                    }
                    Some(api_models::payments::PixAdditionalDetails::Scheduled(val)) => {
                        let cal = SantanderPixRequestCalendar::Scheduled(
                            SantanderPixDueDateCalendarRequest {
                                data_de_vencimento: val.date.clone(),
                                validade_apos_vencimento: val.validity_after_expiration,
                            },
                        );
                        Some(cal)
                    }
                    None => {
                        let cal = SantanderPixRequestCalendar::Immediate(
                            SantanderPixImmediateCalendarRequest { expiracao: 3600 },
                        );

                        Some(cal)
                    }
                };

                // for now we are just updating the expiry, if asked we need to include amount, address in Update Metadata PaymentsRequest and consume from PaymentsUpdateMetadataData
                Ok(Self {
                    calendario: calendar,
                    devedor: None,
                    valor: None,
                    chave: None,
                    solicitacao_pagador: None,
                    info_adicionais: None,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                crate::utils::get_unimplemented_payment_method_error_message("Santander"),
            ))?,
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            SantanderUpdateMetadataResponse,
            PaymentsUpdateMetadataData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsUpdateMetadataData, PaymentsResponseData>
where
    F: Clone,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            SantanderUpdateMetadataResponse,
            PaymentsUpdateMetadataData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = if item.http_code == 200 {
            common_enums::PaymentResourceUpdateStatus::Success
        } else {
            common_enums::PaymentResourceUpdateStatus::Failure
        };
        match item.response {
            SantanderUpdateMetadataResponse::Pix(_) => Ok(Self {
                response: Ok(PaymentsResponseData::PaymentResourceUpdateResponse { status }),
                ..item.data
            }),
            SantanderUpdateMetadataResponse::Boleto(_) => Ok(Self {
                response: Ok(PaymentsResponseData::PaymentResourceUpdateResponse { status }),
                ..item.data
            }),
        }
    }
}

pub fn get_qr_code_type(
    metadata: Option<Value>,
) -> CustomResult<ExpiryType, errors::ConnectorError> {
    let qr_data_santander: Option<QrCodeInformation> = metadata
        .clone()
        .map(|qr_code_data| qr_code_data.parse_value("QrDataUrlSantander"))
        .transpose()
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

    let santander_variant = match qr_data_santander {
        Some(QrCodeInformation::QrCodeImageUrl { expiry_type, .. }) => expiry_type,
        _ => {
            return Err(errors::ConnectorError::ResponseDeserializationFailed.into());
        }
    };
    Ok(santander_variant.ok_or_else(|| errors::ConnectorError::ResponseDeserializationFailed)?)
}

fn extract_boleto_components(input: &str) -> Result<NsuComposite, errors::ConnectorError> {
    let parts: Vec<&str> = input.split('.').collect();

    let [nsu_code, nsu_date, environment, covenant_code, bank_number] = parts
        .as_slice()
        .try_into()
        .map_err(|_| errors::ConnectorError::ParsingFailed)?;

    Ok(NsuComposite {
        nsu_code: nsu_code.to_string(),
        nsu_date: nsu_date.to_string(),
        environment: environment.to_string(),
        covenant_code: covenant_code.to_string(),
        bank_number: bank_number.to_string(),
    })
}
