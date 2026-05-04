use api_models::payments::{
    AccountType, BeneficiaryDetails, BoletoPaymentTypeConstraints, CalculationType,
    ConnectorMetadata, DiscountTier, DiscountType, PollConfig, ProtestType, QrCodeInformation,
    SantanderData, SantanderMandatePeriodicity, SantanderPaymentDiscountRules, VoucherNextStepData,
};
use common_enums::{enums, AttemptStatus, BoletoDocumentKind, ExpiryType, PixKey};
use common_utils::{
    errors::CustomResult,
    ext_traits::{Encode, ValueExt},
    types::{AmountConvertor, StringMajorUnit, StringMajorUnitForConnector},
};
use crc::{Algorithm, Crc};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, BoletoVoucherData, PaymentMethodData, VoucherData},
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::{
        payments::PushNotification, AuthorizeSessionToken, GenerateQr, SetupMandate,
    },
    router_request_types::{
        AuthorizeSessionTokenData, CurrentFlowInfo, GenerateQrRequestData,
        PaymentsUpdateMetadataData, PushNotificationRequestData, ResponseId,
        SetupMandateRequestData,
    },
    router_response_types::{MandateReference, PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsPushNotificationRouterData,
        PaymentsSyncRouterData, PaymentsUpdateMetadataRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors::{self},
};
use hyperswitch_masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::{
    connectors::santander::{
        requests::{
            AccessTokenUrlPath, BoletoAdditionalFields, Discount, DiscountObject, Environment,
            JourneyData, Periodicidade, RecurrenceActivation, RecurrenceCalendar, RecurrenceDebtor,
            RecurrenceLink, RecurrenceValue, RetryPolicy, SantanderAccountType,
            SantanderAuthRequest, SantanderAuthType, SantanderBoletoCancelOperation,
            SantanderBoletoCancelRequest, SantanderBoletoPaymentRequest,
            SantanderBoletoUpdateRequest, SantanderDebtor, SantanderDiscountType,
            SantanderGrantType, SantanderMetadataObject, SantanderPaymentRequest,
            SantanderPaymentsCancelRequest, SantanderPixAutomaticCalendarRequest,
            SantanderPixAutomaticDestinationRequest, SantanderPixAutomaticSolicitationRequest,
            SantanderPixAutomaticoCobrCalendario, SantanderPixAutomaticoCobrRequest,
            SantanderPixAutomaticoCobrValor, SantanderPixAutomaticoRecebedor,
            SantanderPixCancelRequest, SantanderPixDueDateCalendarRequest,
            SantanderPixImmediateCalendarRequest, SantanderPixQRPaymentRequest,
            SantanderPixRequestCalendar, SantanderPostProcessingStepRequest, SantanderProtestType,
            SantanderRefundRequest, SantanderRouterData, SantanderSetupMandateRequest,
            SantanderValue, SantanderValueType,
        },
        responses::{
            Beneficiary, Key, NsuComposite, Payer, RecurrenceStatus, SanatanderAccessTokenResponse,
            SanatanderTokenResponse, SantanderAdditionalInfo, SantanderBoletoDocumentKind,
            SantanderBoletoPaymentType, SantanderBoletoStatus,
            SantanderCreatePixPayloadLocationResponse, SantanderDocumentKind, SantanderJourneyType,
            SantanderPaymentStatus, SantanderPaymentsResponse, SantanderPaymentsSyncResponse,
            SantanderPixAutomaticRecResponse, SantanderPixAutomaticSolicitationResponse,
            SantanderPixAutomaticoCobrStatus, SantanderPixAutomaticoCobrSyncResponse,
            SantanderPixKeyType, SantanderPixQRCodePaymentsResponse,
            SantanderPixQRCodeSyncResponse, SantanderRefundResponse, SantanderRefundStatus,
            SantanderSetupMandateResponse, SantanderUpdateMetadataResponse, SantanderVoidResponse,
            SantanderVoidStatus, WaitScreenData,
        },
    },
    types::{RefreshTokenRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self as connector_utils, PaymentsAuthorizeRequestData, QrImage, RouterData as RouterDataExt,
    },
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

impl
    TryFrom<
        ResponseRouterData<
            AuthorizeSessionToken,
            SantanderCreatePixPayloadLocationResponse,
            AuthorizeSessionTokenData,
            PaymentsResponseData,
        >,
    > for RouterData<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            AuthorizeSessionToken,
            SantanderCreatePixPayloadLocationResponse,
            AuthorizeSessionTokenData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            session_token: Some(item.response.id.to_string()),
            response: Ok(PaymentsResponseData::SessionTokenResponse {
                session_token: item.response.id.to_string(),
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            PushNotification,
            SantanderPixAutomaticSolicitationResponse,
            PushNotificationRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<PushNotification, PushNotificationRequestData, PaymentsResponseData>
{
    type Error = Error;

    fn try_from(
        item: ResponseRouterData<
            PushNotification,
            SantanderPixAutomaticSolicitationResponse,
            PushNotificationRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let expires_in_secs = item
            .data
            .request
            .feature_metadata
            .as_ref()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "feature_metadata",
            })?
            .get_pix_automatico_push_expiry_time()
            .change_context(errors::ConnectorError::ParsingFailed)
            .attach_printable("Failed to get pix_automatico_push expiry time")?;

        let metadata = get_wait_screen_metadata(u64::from(expires_in_secs))?;

        let status = item
            .response
            .status
            .map(AttemptStatus::from)
            .unwrap_or(AttemptStatus::Pending);
        let resource_id =
            ResponseId::ConnectorTransactionId(item.data.connector_request_reference_id.clone());
        let connector_response_reference_id = Some(item.response.id_rec.clone().expose());
        let mandate_reference = Some(MandateReference {
            connector_mandate_id: Some(item.response.id_rec.clone().expose()),
            payment_method_id: None,
            mandate_metadata: None,
            connector_mandate_request_reference_id: Some(item.response.id_solic_rec.expose()),
        });

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata: metadata,
                network_txn_id: None,
                connector_response_reference_id,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            GenerateQr,
            SantanderPixAutomaticRecResponse,
            GenerateQrRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<GenerateQr, GenerateQrRequestData, PaymentsResponseData>
{
    type Error = Error;

    fn try_from(
        item: ResponseRouterData<
            GenerateQr,
            SantanderPixAutomaticRecResponse,
            GenerateQrRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::from(item.response.status);
        let journey = item
            .response
            .dados_qr
            .as_ref()
            .map(|qr_data| qr_data.jornada.clone())
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "response.dadosQR.jornada",
            })?;
        let expiry_type = journey.and_then(Option::<ExpiryType>::from);
        let connector_metadata = match item
            .response
            .dados_qr
            .as_ref()
            .and_then(|dados_qr| dados_qr.pix_copia_e_cola.clone())
        {
            Some(pix_copia_e_cola) => convert_pix_data_to_value(pix_copia_e_cola, expiry_type)?,
            None => None,
        };
        let mandate_reference = Box::new(Some(MandateReference {
            connector_mandate_id: Some(item.response.id_rec.clone().expose()),
            payment_method_id: None,
            mandate_metadata: None,
            connector_mandate_request_reference_id: None,
        }));
        let resource_id =
            ResponseId::ConnectorTransactionId(item.data.connector_request_reference_id.clone());
        let connector_response_reference_id = Some(item.response.id_rec.clone().expose());

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data: Box::new(None),
                mandate_reference,
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
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

        let due_date = Some(format_as_date_only(
            item.request
                .feature_metadata
                .clone()
                .and_then(|data| data.boleto_additional_details)
                .and_then(|boleto_details| boleto_details.due_date),
        )?);

        Ok(Self {
            covenant_code: boleto_mca_metadata.covenant_code,
            bank_number: boleto_components.bank_number,
            due_date,
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
            ConnectorAuthType::CertificateAuth {
                certificate,
                private_key,
            } => Ok(Self {
                client_id: certificate.to_owned(),
                client_secret: private_key.to_owned(),
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
            Some(enums::PaymentMethodType::PixAutomaticoPush) => {
                let pix_mca_metadata = item
                    .1
                    .pix_automatico_push
                    .as_ref()
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)?;
                Ok((
                    pix_mca_metadata.client_id.clone(),
                    pix_mca_metadata.client_secret.clone(),
                ))
            }
            Some(enums::PaymentMethodType::PixAutomaticoQr) => {
                let pix_mca_metadata = item
                    .1
                    .pix_automatico_qr
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
                SanatanderTokenResponse::PixAutomaticoBoleto(boleto_response) => Ok(Self {
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
        // Check if this is a MIT (Merchant Initiated Transaction) for PixAutomaticoPush or PixAutomaticoQr
        if value.router_data.request.is_mit_payment()
            && matches!(
                value.router_data.request.payment_method_type,
                Some(enums::PaymentMethodType::PixAutomaticoPush)
                    | Some(enums::PaymentMethodType::PixAutomaticoQr)
            )
        {
            // Handle MIT recurring charge creation via cobr endpoint
            return Ok(Self::PixAutomaticoCobr(Box::new(
                SantanderPixAutomaticoCobrRequest::try_from(value)?,
            )));
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

        let nsu_code = Some(value.0.router_data.connector_request_reference_id.clone());

        let bank_number = nsu_code.clone();

        let due_date = Some(
            value
                .0
                .router_data
                .request
                .feature_metadata
                .as_ref()
                .and_then(|fm| fm.boleto_additional_details.as_ref())
                .and_then(|details| details.due_date)
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "feature_metadata.boleto_additional_details.due_date",
                })?,
        );

        let covenant_code = value
            .0
            .router_data
            .request
            .feature_metadata
            .clone()
            .and_then(|data| {
                data.get_optional_boleto_covenant_code()
                    .or(Some(boleto_mca_metadata.covenant_code.clone()))
            });

        let key = Some(Key {
            key_type: value
                .0
                .router_data
                .request
                .feature_metadata
                .as_ref()
                .and_then(|data| {
                    data.get_boleto_pix_key_and_value()
                        .0
                        .map(SantanderPixKeyType::from)
                })
                .or(boleto_mca_metadata.pix_key_type),
            dict_key: value
                .0
                .router_data
                .request
                .feature_metadata
                .as_ref()
                .and_then(|data| data.get_boleto_pix_key_and_value().1)
                .or(boleto_mca_metadata.pix_key_value.clone()),
        });

        let messages = value
            .0
            .router_data
            .request
            .billing_descriptor
            .clone()
            .and_then(|data| {
                data.statement_descriptor.map(|s| {
                    vec![
                        s,
                        value.0.router_data.description.clone().unwrap_or_default(),
                    ]
                })
            });

        let customer_document_details = value
            .0
            .router_data
            .customer_document_details
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "customer.document_details",
            })?;

        let (document_type, document_number) = match customer_document_details.document_type {
            common_types::customers::DocumentKind::Cpf => (
                SantanderDocumentKind::Cpf,
                Some(customer_document_details.document_number),
            ),
            common_types::customers::DocumentKind::Cnpj => (
                SantanderDocumentKind::Cnpj,
                Some(customer_document_details.document_number),
            ),
        };

        let order_id = value
            .0
            .router_data
            .request
            .merchant_order_reference_id
            .clone();

        let payer = Some(Payer {
            name: value.0.router_data.get_billing_full_name()?,
            document_type,
            document_number,
            address: Some(Secret::new(
                [
                    value.0.router_data.get_billing_line1()?,
                    value.0.router_data.get_billing_line2()?,
                ]
                .map(|s| s.expose())
                .join(" "),
            )),
            neighborhood: Some(value.0.router_data.get_billing_line1()?),
            city: Some(Secret::new(value.0.router_data.get_billing_city()?)),
            state: Some(value.0.router_data.get_billing_state()?),
            zip_code: Some(value.0.router_data.get_billing_zip()?),
        });

        let (
            (beneficiary, discount, document_kind),
            (fine_percentage, fine_quantity_days, interest_percentage, iof_percentage),
            (protest_type, protest_quantity_days, write_off_quantity_days),
            (
                payment_type,
                value_type,
                parcels_quantity,
                min_value_or_percentage,
                max_value_or_percentage,
            ),
        ) = get_boleto_additional_fields_from_connector_metadata(
            value
                .0
                .router_data
                .request
                .connector_intent_metadata
                .clone(),
        );

        Ok(Self::Boleto(Box::new(SantanderBoletoPaymentRequest {
            environment: Some(Environment::Producao),
            nsu_code,
            nsu_date: Some(
                time::OffsetDateTime::now_utc()
                    .date()
                    .format(&time::macros::format_description!("[year]-[month]-[day]"))
                    .change_context(errors::ConnectorError::DateFormattingFailed)?,
            ),
            covenant_code,
            bank_number,
            client_number: order_id.clone(),
            due_date: Some(format_as_date_only(due_date)?),
            issue_date: Some(
                time::OffsetDateTime::now_utc()
                    .date()
                    .format(&time::macros::format_description!("[year]-[month]-[day]"))
                    .change_context(errors::ConnectorError::DateFormattingFailed)?,
            ),
            nominal_value: Some(value.0.amount.to_owned()),
            participant_code: order_id,
            payer,
            beneficiary,
            document_kind,
            discount,
            fine_percentage,
            fine_quantity_days,
            interest_percentage,
            protest_type,
            protest_quantity_days,
            write_off_quantity_days,
            payment_type,
            parcels_quantity,
            value_type,
            min_value_or_percentage,
            max_value_or_percentage,
            iof_percentage,
            deduction_value: None,
            sharing: None,
            tx_id: None,
            key,
            messages,
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
        // Regular Pix QR payment flow
        let santander_mca_metadata =
            SantanderMetadataObject::try_from(&value.0.router_data.connector_meta_data)?;

        let mca_chave = match value.0.router_data.payment_method_type {
            Some(enums::PaymentMethodType::Pix) => Some(
                santander_mca_metadata
                    .pix
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)
                    .attach_printable("Failed to get pix mca metadata")?
                    .pix_key_value,
            ),
            Some(enums::PaymentMethodType::PixAutomaticoPush) => Some(
                santander_mca_metadata
                    .pix_automatico_push
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)
                    .attach_printable("Failed to get pix automatico push mca metadata")?
                    .pix_key_value,
            ),
            Some(enums::PaymentMethodType::PixAutomaticoQr) => Some(
                santander_mca_metadata
                    .pix_automatico_qr
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)
                    .attach_printable("Failed to get pix automatico qr mca metadata")?
                    .pix_key_value,
            ),
            _ => None,
        };

        let customer_document_details = value
            .0
            .router_data
            .customer_document_details
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "customer.document_details",
            })?;
        let (cpf, cnpj) = match customer_document_details.document_type {
            common_types::customers::DocumentKind::Cpf => {
                (Some(customer_document_details.document_number), None)
            }
            common_types::customers::DocumentKind::Cnpj => {
                (None, Some(customer_document_details.document_number))
            }
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
                    cnpj,
                    nome: Some(value.0.router_data.get_billing_full_name()?),
                    logradouro: None,
                    cidade: None,
                    uf: None,
                    cep: None,
                    cpf,
                });

                (Some(cal), debt)
            }
            Some(api_models::payments::PixAdditionalDetails::Scheduled(val)) => {
                let cal =
                    SantanderPixRequestCalendar::Scheduled(SantanderPixDueDateCalendarRequest {
                        data_de_vencimento: format_as_date_only(Some(val.date))?,
                        validade_apos_vencimento: val.validity_after_expiration,
                    });

                let debt = Some(SantanderDebtor {
                    cpf,
                    nome: Some(value.0.router_data.get_billing_full_name()?),
                    logradouro: None,
                    cidade: None,
                    uf: None,
                    cep: None,
                    cnpj,
                });

                (Some(cal), debt)
            }
            None => {
                let cal =
                    SantanderPixRequestCalendar::Immediate(SantanderPixImmediateCalendarRequest {
                        expiracao: 3600, // default 1 hour
                    });

                let debt = Some(SantanderDebtor {
                    cnpj,
                    nome: Some(value.0.router_data.get_billing_full_name()?),
                    logradouro: None,
                    cidade: None,
                    uf: None,
                    cep: None,
                    cpf,
                });

                (Some(cal), debt)
            }
        };

        let info_adicionais = value
            .0
            .router_data
            .request
            .metadata
            .as_ref()
            .and_then(|m| m.as_object())
            .map(|m| {
                m.iter()
                    .map(|(k, v)| SantanderAdditionalInfo {
                        nome: k.clone().into(),
                        valor: v.as_str().unwrap_or_default().to_string(),
                    })
                    .collect::<Vec<_>>()
            });

        let chave = value
            .0
            .router_data
            .request
            .feature_metadata
            .clone()
            .and_then(|data| data.get_pix_key_and_value().1)
            .or(mca_chave);

        Ok(Self::PixQR(Box::new(SantanderPixQRPaymentRequest {
            calendario: calendar,
            devedor: debtor,
            valor: Some(SantanderValue {
                original: value.0.amount.to_owned(),
            }),
            chave,
            solicitacao_pagador: value
                .0
                .router_data
                .request
                .billing_descriptor
                .clone()
                .and_then(|data| data.statement_descriptor),
            info_adicionais,
        })))
    }
}

impl TryFrom<&SantanderRouterData<&PaymentsAuthorizeRouterData>>
    for SantanderPixAutomaticoCobrRequest
{
    type Error = Error;
    fn try_from(
        value: &SantanderRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let mit_data = value
            .router_data
            .request
            .connector_intent_metadata
            .clone()
            .and_then(|m| m.santander)
            .and_then(|santander| santander.pix_automatico)
            .and_then(|pix_automatico| match pix_automatico {
                api_models::payments::SantanderPixAutomaticoData::Mit(mit) => Some(mit),
                _ => None,
            })
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_metadata.santander.pix_automatico.mit",
            })?;

        let receiver_details =
            mit_data
                .receiver_details
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "connector_metadata.santander.pix_automatico.mit.receiver_details",
                })?;

        let branch_code =
            receiver_details
                .branch_code
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name:
                    "connector_metadata.santander.pix_automatico.mit.receiver_details.branch_code",
            })?;

        let account_number = receiver_details
            .account_number
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
            field_name:
                "connector_metadata.santander.pix_automatico.mit.receiver_details.account_number",
        })?;

        let account_type =
            receiver_details
                .account_type
                .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name:
                    "connector_metadata.santander.pix_automatico.mit.receiver_details.account_type",
            })?;

        let recebedor = SantanderPixAutomaticoRecebedor {
            agencia: branch_code,
            conta: account_number,
            tipo_conta: Some(SantanderAccountType::from(account_type)),
        };

        let id_rec = value.router_data.request.get_connector_mandate_id()?;

        // Use mandate_execution_date from MIT data if provided, otherwise default to current date + 1 day
        let due_date = match mit_data.mandate_execution_date {
            Some(exec_date) => format_as_date_only(Some(exec_date))?,
            None => time::OffsetDateTime::now_utc()
                .checked_add(time::Duration::days(1))
                .ok_or(errors::ConnectorError::DateFormattingFailed)?
                .date()
                .format(&time::macros::format_description!("[year]-[month]-[day]"))
                .change_context(errors::ConnectorError::DateFormattingFailed)?,
        };

        let calendario = SantanderPixAutomaticoCobrCalendario {
            data_de_vencimento: due_date,
        };

        let ajuste_dia_util = mit_data.auto_adjust_date.unwrap_or(true);

        let valor = SantanderPixAutomaticoCobrValor {
            original: value.amount.to_owned(),
        };

        let info_adicional = value.router_data.description.clone().or_else(|| {
            value
                .router_data
                .request
                .billing_descriptor
                .as_ref()
                .and_then(|bd| bd.statement_descriptor.clone())
        });

        let devedor = Some(SantanderDebtor {
            cpf: None,
            cnpj: None,
            nome: Some(value.router_data.get_billing_full_name()?),
            logradouro: None,
            cidade: None,
            uf: None,
            cep: None,
        });

        Ok(Self {
            id_rec: id_rec.into(),
            info_adicional,
            calendario,
            valor,
            ajuste_dia_util,
            recebedor,
            devedor,
        })
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

// Helper function to convert SantanderPixAutomaticoCobrStatus to AttemptStatus with MIT flag
fn cobr_status_to_attempt_status(
    status: SantanderPixAutomaticoCobrStatus,
    is_mit: bool,
) -> AttemptStatus {
    match status {
        SantanderPixAutomaticoCobrStatus::Criada => match is_mit {
            false => AttemptStatus::AuthenticationPending,
            true => AttemptStatus::Pending,
        },
        SantanderPixAutomaticoCobrStatus::Ativa => AttemptStatus::AuthenticationPending,
        SantanderPixAutomaticoCobrStatus::Concluida => AttemptStatus::Charged,
        SantanderPixAutomaticoCobrStatus::Expirada => AttemptStatus::Failure,
        SantanderPixAutomaticoCobrStatus::Rejeitada => AttemptStatus::Failure,
        SantanderPixAutomaticoCobrStatus::Cancelada => AttemptStatus::Voided,
    }
}

/// Helper function to determine AttemptStatus from cobr sync response.
/// For CONCLUIDA status, only marks as Charged if endToEndId is present in the pix array.
fn cobr_sync_status_to_attempt_status(
    cobr_data: &SantanderPixAutomaticoCobrSyncResponse,
) -> AttemptStatus {
    match cobr_data.status {
        SantanderPixAutomaticoCobrStatus::Criada => AttemptStatus::Pending,
        SantanderPixAutomaticoCobrStatus::Ativa => AttemptStatus::Pending,
        SantanderPixAutomaticoCobrStatus::Concluida => {
            // Only mark as Charged if endToEndId is present in the pix object
            let has_end_to_end_id = cobr_data
                .pix
                .as_ref()
                .and_then(|pix_list| pix_list.first())
                .map(|pix| !pix.end_to_end_id.clone().expose().is_empty())
                .unwrap_or(false);
            if has_end_to_end_id {
                AttemptStatus::Charged
            } else {
                AttemptStatus::Failure
            }
        }
        SantanderPixAutomaticoCobrStatus::Expirada => AttemptStatus::Failure,
        SantanderPixAutomaticoCobrStatus::Rejeitada => AttemptStatus::Failure,
        SantanderPixAutomaticoCobrStatus::Cancelada => AttemptStatus::Voided,
    }
}

impl From<SantanderBoletoStatus> for AttemptStatus {
    fn from(item: SantanderBoletoStatus) -> Self {
        match item {
            SantanderBoletoStatus::Ativo => Self::AuthenticationPending,
            SantanderBoletoStatus::Baixado => Self::Voided,
            SantanderBoletoStatus::Liquidado => Self::Charged,
            SantanderBoletoStatus::LiquidadoParcialmente => Self::PartialChargedAndChargeable,
        }
    }
}

impl From<RecurrenceStatus> for AttemptStatus {
    fn from(item: RecurrenceStatus) -> Self {
        match item {
            RecurrenceStatus::Criada => Self::AuthenticationPending,
            RecurrenceStatus::Aprovada => Self::Charged,
            RecurrenceStatus::Rejeitada => Self::Failure,
            RecurrenceStatus::Expirada => Self::Failure,
            RecurrenceStatus::Cancelada => Self::Voided,
            RecurrenceStatus::Recebida | RecurrenceStatus::Aceita | RecurrenceStatus::Enviada => {
                Self::Pending
            }
            _ => Self::Pending,
        }
    }
}

impl From<common_types::customers::DocumentKind> for SantanderDocumentKind {
    fn from(item: common_types::customers::DocumentKind) -> Self {
        match item {
            common_types::customers::DocumentKind::Cnpj => Self::Cnpj,
            common_types::customers::DocumentKind::Cpf => Self::Cpf,
        }
    }
}

impl From<PixKey> for SantanderPixKeyType {
    fn from(item: PixKey) -> Self {
        match item {
            PixKey::Cpf(_) => Self::Cpf,
            PixKey::Cnpj(_) => Self::Cnpj,
            PixKey::Email(_) => Self::Email,
            PixKey::Phone(_) => Self::Cellular,
            PixKey::EvpToken(_) => Self::Evp,
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
            // Journey 3/4
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
                            .as_ref()
                            .and_then(|pix_list| pix_list.first())
                            .map(|pix| {
                                let data = SantanderData {
                                    end_to_end_id: Some(pix.end_to_end_id.clone().expose()),
                                };
                                serde_json::to_value(data)
                                    .change_context(errors::ConnectorError::ParsingFailed)
                            })
                            .transpose()?;
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
                                authentication_data: None,
                                charges: None,
                            }),
                            ..item.data
                        })
                    }
                }
            }
            // Journey 1/2
            SantanderPaymentsSyncResponse::PixAutomaticoConsultAndActivateJourney(res) => {
                let status = AttemptStatus::from(res.status);
                // TODO : Make a write on connector_metadata to modify the ExpiryType as it may chance after approval of Recurrence
                Ok(Self {
                    status,
                    response: item.data.response,
                    ..item.data
                })
            }
            SantanderPaymentsSyncResponse::Boleto(res) => {
                let status = res.content.first().map(|data| data.status.clone()).ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "status",
                    },
                )?;
                Ok(Self {
                    status: AttemptStatus::from(status),
                    response: item.data.response,
                    ..item.data
                })
            }
            // MIT recurring charge sync (cobr endpoint consultation)
            SantanderPaymentsSyncResponse::PixAutomaticoCobrSync(cobr_data) => {
                let attempt_status = cobr_sync_status_to_attempt_status(&cobr_data);
                let connector_metadata = cobr_data
                    .pix
                    .as_ref()
                    .and_then(|pix_list| pix_list.first())
                    .map(|pix| {
                        let data = SantanderData {
                            end_to_end_id: Some(pix.end_to_end_id.clone().expose()),
                        };
                        serde_json::to_value(data)
                            .change_context(errors::ConnectorError::ParsingFailed)
                    })
                    .transpose()?;
                Ok(Self {
                    status: attempt_status,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(cobr_data.txid.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata,
                        network_txn_id: None,
                        connector_response_reference_id: Some(cobr_data.id_rec.expose()),
                        incremental_authorization_allowed: None,
                        authentication_data: None,
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
                    _ => {
                        let is_pmt_automatico = matches!(
                            item.data.payment_method_type,
                            Some(enums::PaymentMethodType::PixAutomaticoQr)
                                | Some(enums::PaymentMethodType::PixAutomaticoPush)
                        );
                        let connector_metadata = if !is_pmt_automatico {
                            get_qr_code_data(&item, &pix_data)?
                        } else {
                            None
                        };
                        Ok(Self {
                            status: AttemptStatus::from(pix_data.status.clone()),
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
                                authentication_data: None,
                                charges: None,
                            }),
                            ..item.data
                        })
                    }
                }
            }
            SantanderPaymentsResponse::Boleto(boleto_data) => {
                let qr_code_url = if let Some(data) = boleto_data.qr_code_pix.clone() {
                    let qr_image = QrImage::new_from_data(data)
                        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
                    let url_str = &qr_image.data;
                    Some(
                        Url::parse(url_str)
                            .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
                    )
                } else {
                    None
                };

                let voucher_data = VoucherNextStepData {
                    digitable_line: boleto_data.digitable_line.clone(),
                    barcode: boleto_data.bar_code.clone(),
                    expires_at: None,
                    expiry_date: Some(boleto_data.due_date),
                    reference: boleto_data.nsu_code.clone(),
                    raw_qr_data: boleto_data.qr_code_pix.clone(),
                    entry_date: boleto_data.entry_date.clone(),
                    download_url: None,
                    instructions_url: None,
                    qr_code_url,
                };

                let connector_metadata = Some(voucher_data.encode_to_value())
                    .transpose()
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

                Ok(Self {
                    status: AttemptStatus::AuthenticationPending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            boleto_data.bank_number.clone(),
                        ),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        authentication_data: None,
                        charges: None,
                    }),
                    ..item.data
                })
            }
            SantanderPaymentsResponse::PixAutomaticoCobr(cobr_data) => {
                // passing is_mit as true since this struct comes only when MITs are triggered
                let attempt_status = cobr_status_to_attempt_status(cobr_data.status.clone(), true);
                Ok(Self {
                    status: attempt_status,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(cobr_data.txid.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(Some(MandateReference {
                            connector_mandate_id: Some(cobr_data.id_rec.clone().expose()),
                            payment_method_id: None,
                            mandate_metadata: None,
                            connector_mandate_request_reference_id: Some(cobr_data.txid.clone()),
                        })),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(cobr_data.id_rec.clone().expose()),
                        incremental_authorization_allowed: None,
                        authentication_data: None,
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
        match item.response.clone() {
            SantanderVoidResponse::Pix(res) => Ok(Self {
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
                    authentication_data: None,
                }),
                ..item.data
            }),
            SantanderVoidResponse::Boleto(_) => Ok(Self {
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

    let qr_code_info = QrCodeInformation::QrCodeUrl {
        image_data_url: image_data_url.clone(),
        qr_code_url: image_data_url,
        display_to_timestamp: None,
        expiry_type: variant,
        raw_qr_data: Some(data),
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
            SantanderRefundStatus::Devolvido => Self::Success,
            SantanderRefundStatus::NaoRealizado => Self::Failure,
            SantanderRefundStatus::EmProcessamento => Self::Pending,
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

impl TryFrom<&PaymentsUpdateMetadataRouterData> for SantanderPaymentRequest {
    type Error = Error;
    fn try_from(value: &PaymentsUpdateMetadataRouterData) -> Result<Self, Self::Error> {
        match value.request.payment_method_type {
            Some(common_enums::PaymentMethodType::Pix) => {
                let pix_qr = SantanderPixQRPaymentRequest::try_from(value)?;
                Ok(Self::PixQR(Box::new(pix_qr)))
            }
            Some(common_enums::PaymentMethodType::Boleto) => {
                let boleto = SantanderBoletoPaymentRequest::try_from(value)?;
                Ok(Self::Boleto(Box::new(boleto)))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                crate::utils::get_unimplemented_payment_method_error_message("Santander"),
            ))?,
        }
    }
}

impl TryFrom<&PaymentsUpdateMetadataRouterData> for SantanderBoletoPaymentRequest {
    type Error = Error;

    fn try_from(value: &PaymentsUpdateMetadataRouterData) -> Result<Self, Self::Error> {
        let santander_mca_metadata = SantanderMetadataObject::try_from(&value.connector_meta_data)?;

        let boleto_mca_metadata = santander_mca_metadata
            .boleto
            .ok_or(errors::ConnectorError::NoConnectorMetaData)?;

        let due_date = Some(
            value
                .request
                .feature_metadata
                .as_ref()
                .and_then(|fm| fm.boleto_additional_details.as_ref())
                .and_then(|details| details.due_date)
                .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                    field_name: "feature_metadata.boleto_additional_details.due_date",
                })?,
        );

        let covenant_code = value.request.feature_metadata.clone().and_then(|data| {
            data.get_optional_boleto_covenant_code()
                .or(Some(boleto_mca_metadata.covenant_code.clone()))
        });

        let key = Some(Key {
            key_type: value
                .request
                .feature_metadata
                .as_ref()
                .and_then(|data| {
                    data.get_boleto_pix_key_and_value()
                        .0
                        .map(SantanderPixKeyType::from)
                })
                .or(boleto_mca_metadata.pix_key_type),
            dict_key: value
                .request
                .feature_metadata
                .as_ref()
                .and_then(|data| data.get_boleto_pix_key_and_value().1)
                .or(boleto_mca_metadata.pix_key_value.clone()),
        });

        Ok(Self {
            bank_number: Some(value.connector_request_reference_id.clone()),
            covenant_code,
            environment: Some(Environment::Producao),
            due_date: Some(format_as_date_only(due_date)?),
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
            protest_type: None,
            protest_quantity_days: None,
            write_off_quantity_days: None,
            payment_type: None,
            parcels_quantity: None,
            value_type: None,
            min_value_or_percentage: None,
            max_value_or_percentage: None,
            iof_percentage: None,
            deduction_value: None,
            sharing: None,
            key,
            tx_id: None,
            messages: None,
        })
    }
}

impl TryFrom<&PaymentsUpdateMetadataRouterData> for SantanderPixQRPaymentRequest {
    type Error = Error;

    fn try_from(value: &PaymentsUpdateMetadataRouterData) -> Result<Self, Self::Error> {
        match value.request.payment_method_type {
            Some(common_enums::PaymentMethodType::Pix) => {
                let santander_mca_metadata =
                    SantanderMetadataObject::try_from(&value.connector_meta_data)?;
                let pix_mca_metadata = santander_mca_metadata
                    .pix
                    .ok_or(errors::ConnectorError::NoConnectorMetaData)?;
                let calendar = match &value
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
                                data_de_vencimento: format_as_date_only(Some(val.date))?,
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

                let chave = value
                    .request
                    .feature_metadata
                    .clone()
                    .and_then(|data| data.get_pix_key_and_value().1)
                    .or(Some(pix_mca_metadata.pix_key_value.clone()));

                Ok(Self {
                    calendario: calendar,
                    devedor: None,
                    valor: None,
                    chave,
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

pub fn get_qr_code_type(metadata: Option<Value>) -> Option<ExpiryType> {
    let qr_data_santander: Option<QrCodeInformation> =
        metadata.and_then(|qr_code_data| qr_code_data.parse_value("QrDataUrlSantander").ok());

    match qr_data_santander {
        Some(QrCodeInformation::QrCodeUrl { expiry_type, .. }) => expiry_type,
        _ => None,
    }
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

pub fn format_as_date_only(
    date_time: Option<time::PrimitiveDateTime>,
) -> Result<String, errors::ConnectorError> {
    let dt = date_time.ok_or(errors::ConnectorError::MissingRequiredField {
        field_name: "due_date",
    })?;

    let format = time::macros::format_description!("[year]-[month]-[day]");
    dt.format(&format)
        .map_err(|_| errors::ConnectorError::ParsingFailed)
}

impl From<BeneficiaryDetails> for Beneficiary {
    fn from(b: BeneficiaryDetails) -> Self {
        Self {
            name: b.name.map(Secret::new),
            document_type: b.document_type.map(Into::into),
            document_number: b.document_number.map(Secret::new),
        }
    }
}

impl From<SantanderPaymentDiscountRules> for Discount {
    fn from(rules: SantanderPaymentDiscountRules) -> Self {
        let mut tiers = rules.tiers.into_iter();
        let map_tier = |tier: DiscountTier| -> DiscountObject {
            DiscountObject {
                value: tier.amount,
                limit_date: tier.end_date,
            }
        };
        Self {
            discount_type: SantanderDiscountType::from(rules.discount_type.unwrap_or_default()),
            discount_one: tiers.next().map(map_tier),
            discount_two: tiers.next().map(map_tier),
            discount_three: tiers.next().map(map_tier),
        }
    }
}

impl From<DiscountType> for SantanderDiscountType {
    fn from(value: DiscountType) -> Self {
        match value {
            DiscountType::Standard => Self::Isento,
            DiscountType::FixedDate => Self::ValorDataFixa,
            DiscountType::DailyCalendar => Self::ValorDiaCorrido,
            DiscountType::DailyBusiness => Self::ValorDiaUtil,
        }
    }
}

impl From<ProtestType> for SantanderProtestType {
    fn from(value: ProtestType) -> Self {
        match value {
            ProtestType::Disabled => Self::SemProtesto,
            ProtestType::CalendarDays => Self::DiasCorridos,
            ProtestType::BusinessDays => Self::DiasUteis,
            ProtestType::ContractDefault => Self::CadastroConvenio,
        }
    }
}

impl From<CalculationType> for SantanderValueType {
    fn from(calc_type: CalculationType) -> Self {
        match calc_type {
            CalculationType::Percentage => Self::Percentual,
            CalculationType::FlatAmount => Self::Valor,
        }
    }
}

impl From<BoletoPaymentTypeConstraints> for SantanderBoletoPaymentType {
    fn from(internal_type: BoletoPaymentTypeConstraints) -> Self {
        match internal_type {
            BoletoPaymentTypeConstraints::FixedAmount => Self::Registro,
            BoletoPaymentTypeConstraints::FlexibleAmount(_) => Self::Divergente,
            BoletoPaymentTypeConstraints::Installment(_) => Self::Parcial,
        }
    }
}

fn get_boleto_additional_fields_from_connector_metadata(
    metadata: Option<ConnectorMetadata>,
) -> BoletoAdditionalFields {
    metadata
        .and_then(|m| m.santander)
        .and_then(|s| s.boleto)
        .map(|b| {
            let fine = b.penalties.as_ref().and_then(|p| p.fixed_penalty.as_ref());
            let fine_quantity_days = fine.and_then(|f| f.grace_period_days.map(|d| d.to_string()));
            let fine_percentage = fine.and_then(|f| f.value.clone());

            let (interest_percentage, iof_percentage) = b
                .penalties
                .as_ref()
                .and_then(|p| p.interest.as_ref())
                .map(|i| (i.interest_percentage.clone(), i.iof_percentage.clone()))
                .unwrap_or((None, None));

            let protest = b
                .collection_actions
                .as_ref()
                .and_then(|c| c.legal_protest.as_ref());

            let beneficiary = b.beneficiary.map(Beneficiary::from);
            let discount = b.discount_rules.map(Discount::from);
            let protest_type = protest.and_then(|p| {
                p.protest_type
                    .as_ref()
                    .map(|pt| SantanderProtestType::from(pt.clone()))
            });
            let write_off_quantity_days = b
                .collection_actions
                .as_ref()
                .and_then(|c| c.auto_write_off_days.map(|d| d.to_string()));
            let protest_quantity_days =
                protest.and_then(|p| p.days_after_due_date.map(|d| d.to_string()));
            let document_kind = b.document_kind.map(SantanderBoletoDocumentKind::from);

            let (
                payment_type,
                value_type,
                parcels_quantity,
                min_value_or_percentage,
                max_value_or_percentage,
            ) = match b.payment_constraints {
                Some(ref constraints) => match constraints {
                    BoletoPaymentTypeConstraints::FixedAmount => (
                        Some(SantanderBoletoPaymentType::from(constraints.clone())),
                        None,
                        None,
                        None,
                        None,
                    ),
                    BoletoPaymentTypeConstraints::FlexibleAmount(ref details) => (
                        Some(SantanderBoletoPaymentType::from(constraints.clone())),
                        details
                            .value_type
                            .as_ref()
                            .map(|vt| SantanderValueType::from(vt.clone())),
                        None,
                        details.min_value.clone(),
                        details.max_value.clone(),
                    ),
                    BoletoPaymentTypeConstraints::Installment(ref details) => (
                        Some(SantanderBoletoPaymentType::from(constraints.clone())),
                        details
                            .value_type
                            .as_ref()
                            .map(|vt| SantanderValueType::from(vt.clone())),
                        details.max_partial_payments,
                        None,
                        None,
                    ),
                },
                None => (None, None, None, None, None),
            };

            (
                (beneficiary, discount, document_kind),
                (
                    fine_percentage,
                    fine_quantity_days,
                    interest_percentage,
                    iof_percentage,
                ),
                (protest_type, protest_quantity_days, write_off_quantity_days),
                (
                    payment_type,
                    value_type,
                    parcels_quantity,
                    min_value_or_percentage,
                    max_value_or_percentage,
                ),
            )
        })
        .unwrap_or_default()
}

impl
    TryFrom<
        &SantanderRouterData<
            &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        >,
    > for SantanderSetupMandateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: &SantanderRouterData<
            &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        >,
    ) -> Result<Self, Self::Error> {
        let item = value.router_data;
        let santander_meta = item
            .request
            .connector_intent_metadata
            .as_ref()
            .and_then(|metadata| metadata.santander.as_ref())
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_metadata.santander",
            })?;

        let pix_automatico_meta = santander_meta.pix_automatico.as_ref().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "pix_automatico metadata",
            },
        )?;

        let cit_data = match pix_automatico_meta {
            api_models::payments::SantanderPixAutomaticoData::Cit(cit) => cit,
            _ => {
                return Err(errors::ConnectorError::MissingRequiredField {
                    field_name: "connector_metadata.santander.pix_automatico.cit",
                })?;
            }
        };

        let contrato = cit_data
            .contract_id
            .clone()
            .unwrap_or_else(|| item.payment_id.clone());

        let politica_retentativa = if cit_data.retry_policy.unwrap_or(false) {
            RetryPolicy::Permite3r7d
        } else {
            RetryPolicy::NaoPermite
        };

        let billing_full_name = item.get_optional_billing_full_name();
        let request_customer_name = item.request.customer_name.clone();
        let customer_name = billing_full_name
            .clone()
            .or(request_customer_name.clone())
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name:
                    "billing.address.first_name or billing.address.last_name or customer.name",
            })?;

        let (cpf, cnpj) = item
            .customer_document_details
            .as_ref()
            .map(|details| match details.document_type {
                common_types::customers::DocumentKind::Cpf => {
                    (Some(details.document_number.clone()), None)
                }
                common_types::customers::DocumentKind::Cnpj => {
                    (None, Some(details.document_number.clone()))
                }
            })
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "customer.document_details",
            })?;

        let objeto =
            item.description
                .clone()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "description",
                })?;

        let loc = item
            .session_token
            .as_ref()
            .and_then(|token| token.parse::<i64>().ok());

        let mandate_details = cit_data.mandate_details.as_ref();

        let data_inicial = match mandate_details.and_then(|md| md.start_date) {
            Some(start_date) => format_as_date_only(Some(start_date))?,
            None => time::OffsetDateTime::now_utc()
                .date()
                .format(&time::macros::format_description!("[year]-[month]-[day]"))
                .change_context(errors::ConnectorError::DateFormattingFailed)?,
        };

        let data_final = mandate_details
            .and_then(|md| md.end_date)
            .map(|end_date| format_as_date_only(Some(end_date)))
            .transpose()?;

        let periodicidade = mandate_details
            .and_then(|md| md.periodicity.as_ref())
            .map(|p| Periodicidade::from(p.clone()))
            .unwrap_or(Periodicidade::Semanal);

        // either of valor or valor_minimo_recebedor can be passed at one time
        let valor = Some(RecurrenceValue {
            valor_rec: Some(value.amount.clone()),
            valor_minimo_recebedor: None,
        });

        let is_immediate = item
            .request
            .feature_metadata
            .as_ref()
            .and_then(|f| f.pix_additional_details.as_ref())
            .map(|pix_details| {
                matches!(
                    pix_details,
                    api_models::payments::PixAdditionalDetails::Immediate(_)
                )
            })
            .unwrap_or(false);

        let is_journey_3 = if is_immediate {
            item.request.payment_method_type == Some(enums::PaymentMethodType::PixAutomaticoQr)
                && item.request.amount > 0
        } else {
            false
        };

        // pass ativacao only when Journey 4
        let ativacao = if is_journey_3 {
            Some(RecurrenceActivation {
                dados_jornada: Some(JourneyData {
                    txid: item.connector_request_reference_id.clone(),
                }),
            })
        } else {
            None
        };

        Ok(Self {
            vinculo: RecurrenceLink {
                contrato,
                devedor: RecurrenceDebtor {
                    cpf,
                    cnpj,
                    nome: customer_name,
                },
                objeto,
            },
            calendario: RecurrenceCalendar {
                data_inicial,
                periodicidade,
                data_final,
            },
            politica_retentativa,
            loc,
            valor,
            ativacao,
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            SantanderSetupMandateResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<F, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            F,
            SantanderSetupMandateResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        if !matches!(router_env::env::which(), router_env::env::Env::Production) {
            router_env::logger::info!(
                "Santander Recurrence Id: {:?}",
                item.response.id_rec.clone().expose()
            );
        }
        Ok(Self {
            status: AttemptStatus::from(item.response.status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.data.connector_request_reference_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(Some(MandateReference {
                    connector_mandate_id: Some(item.response.id_rec.expose()),
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: None,
                })),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
                authentication_data: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<&PaymentsPushNotificationRouterData> for SantanderPostProcessingStepRequest {
    type Error = Error;
    fn try_from(value: &PaymentsPushNotificationRouterData) -> Result<Self, Self::Error> {
        match &value.request.payment_method_data {
            Some(PaymentMethodData::BankTransfer(bank_transfer_data)) => {
                match bank_transfer_data.as_ref() {
                    BankTransferData::PixAutomaticoPush { .. } => {
                        let solicitation_request =
                            SantanderPixAutomaticSolicitationRequest::try_from(value)?;
                        Ok(Self::PixAutomaticoPush(solicitation_request))
                    }
                    // For PixAutomaticoQr, since there are no additional details needed in the request body,it should be null
                    BankTransferData::PixAutomaticoQr {} => Ok(Self::PixAutomaticoQr()),
                    _ => Err(errors::ConnectorError::NotImplemented(
                        crate::utils::get_unimplemented_payment_method_error_message("Santander"),
                    ))?,
                }
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                crate::utils::get_unimplemented_payment_method_error_message("Santander"),
            ))?,
        }
    }
}
impl TryFrom<&PaymentsPushNotificationRouterData> for SantanderPixAutomaticSolicitationRequest {
    type Error = Error;

    fn try_from(item: &PaymentsPushNotificationRouterData) -> Result<Self, Self::Error> {
        // Extract expiration time from feature_metadata using the helper function
        let expiry_time_seconds = item
            .request
            .feature_metadata
            .as_ref()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "feature_metadata",
            })?
            .get_pix_automatico_push_expiry_time()
            .change_context(errors::ConnectorError::ParsingFailed)
            .attach_printable("Failed to get pix_automatico_push expiry time")?;

        let expiry_seconds = i64::from(expiry_time_seconds);
        let offset_datetime = time::OffsetDateTime::now_utc()
            .checked_add(time::Duration::seconds(expiry_seconds))
            .ok_or(errors::ConnectorError::ParsingFailed)?;

        // Format as ISO 8601 with Z timezone: YYYY-MM-ddTHH:mm:ss.SSSZ
        let data_expiracao_solicitacao = offset_datetime
            .format(&time::macros::format_description!(
                "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
            ))
            .change_context(errors::ConnectorError::DateFormattingFailed)?;

        // Extract bank transfer data for PixAutomaticoPush
        let bank_transfer_data = match &item.request.payment_method_data {
            Some(PaymentMethodData::BankTransfer(boxed_data)) => match boxed_data.as_ref() {
                BankTransferData::PixAutomaticoPush {
                    account_number,
                    branch_code,
                    bank_identifier,
                } => (account_number, branch_code, bank_identifier),
                _ => {
                    return Err(errors::ConnectorError::MissingRequiredField {
                        field_name: "payment_method_data.bank_transfer.pix_automatico_push",
                    })?
                }
            },
            _ => {
                return Err(errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_method_data.bank_transfer.pix_automatico_push",
                })?
            }
        };

        // Extract customer document details
        let customer_document_details = item.customer_document_details.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "customer.document_details",
            },
        )?;

        let (cpf, cnpj) = match customer_document_details.document_type {
            common_types::customers::DocumentKind::Cpf => {
                (Some(customer_document_details.document_number), None)
            }
            common_types::customers::DocumentKind::Cnpj => {
                (None, Some(customer_document_details.document_number))
            }
        };

        Ok(Self {
            id_rec: item
                .request
                .get_connector_mandate_id()
                .ok_or(errors::ConnectorError::MissingConnectorMandateID)?
                .into(),
            calendario: SantanderPixAutomaticCalendarRequest {
                data_expiracao_solicitacao,
            },
            destinatario: SantanderPixAutomaticDestinationRequest {
                agencia: bank_transfer_data
                    .1
                    .clone()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "payment_method_data.bank_transfer.branch_code",
                    })?
                    .expose(),
                conta: bank_transfer_data
                    .0
                    .clone()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "payment_method_data.bank_transfer.account_number",
                    })?
                    .expose(),
                cpf,
                cnpj,
                ispb_participante: bank_transfer_data
                    .2
                    .clone()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "payment_method_data.bank_transfer.bank_identifier",
                    })?
                    .expose(),
            },
        })
    }
}

fn get_wait_screen_metadata(
    expiry_in_secs: u64,
) -> CustomResult<Option<Value>, errors::ConnectorError> {
    let current_time = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let expiry_duration_nanos = i128::from(expiry_in_secs) * 1_000_000_000;
    // confirm this value from sdk team
    let delay_in_secs: u16 = 5;
    let frequency: u16 = u16::try_from(expiry_in_secs / u64::from(delay_in_secs))
        .change_context(errors::ConnectorError::ParsingFailed)
        .attach_printable("Failed to convert frequency to u16")?;

    Ok(Some(serde_json::json!(WaitScreenData {
        display_from_timestamp: current_time,
        display_to_timestamp: Some(current_time + expiry_duration_nanos),
        poll_config: Some(PollConfig {
            delay_in_secs,
            frequency,
        }),
    })))
}

impl From<AccountType> for SantanderAccountType {
    fn from(account_type: AccountType) -> Self {
        match account_type {
            AccountType::Current => Self::Corrente,
            AccountType::Savings => Self::Poupanca,
            AccountType::Payment => Self::Pagamento,
        }
    }
}

impl From<SantanderMandatePeriodicity> for Periodicidade {
    fn from(item: SantanderMandatePeriodicity) -> Self {
        match item {
            SantanderMandatePeriodicity::Weekly => Self::Semanal,
            SantanderMandatePeriodicity::Monthly => Self::Mensal,
            SantanderMandatePeriodicity::Quarterly => Self::Trimestral,
            SantanderMandatePeriodicity::Semiannually => Self::Semestral,
            SantanderMandatePeriodicity::Annually => Self::Anual,
        }
    }
}

pub fn decide_access_token_key_suffix(
    current_flow_info: Option<CurrentFlowInfo>,
    payment_method_type: Option<enums::PaymentMethodType>,
    is_mit: bool,
) -> Option<AccessTokenUrlPath> {
    match is_mit {
        true => Some(AccessTokenUrlPath::Leg2),
        false => {
            match (current_flow_info, payment_method_type) {
                // Authorize flow
                (
                    Some(CurrentFlowInfo::Authorize { .. }),
                    Some(enums::PaymentMethodType::Boleto),
                ) => Some(AccessTokenUrlPath::Boleto),
                (Some(CurrentFlowInfo::Authorize { .. }), Some(enums::PaymentMethodType::Pix)) => {
                    Some(AccessTokenUrlPath::Leg1)
                }
                (
                    Some(CurrentFlowInfo::Authorize { .. }),
                    Some(enums::PaymentMethodType::PixAutomaticoPush),
                ) => {
                    // redundant case
                    Some(AccessTokenUrlPath::Leg2)
                }
                (
                    Some(CurrentFlowInfo::Authorize { .. }),
                    Some(enums::PaymentMethodType::PixAutomaticoQr),
                ) => Some(AccessTokenUrlPath::Leg1),
                // CompleteAuthorize flow
                (
                    Some(CurrentFlowInfo::CompleteAuthorize { .. }),
                    Some(enums::PaymentMethodType::Boleto),
                ) => Some(AccessTokenUrlPath::Boleto),
                (
                    Some(CurrentFlowInfo::CompleteAuthorize { .. }),
                    Some(enums::PaymentMethodType::Pix),
                ) => Some(AccessTokenUrlPath::Leg1),
                (
                    Some(CurrentFlowInfo::CompleteAuthorize { .. }),
                    Some(enums::PaymentMethodType::PixAutomaticoPush),
                ) => {
                    // redundant case
                    Some(AccessTokenUrlPath::Leg2)
                }
                (
                    Some(CurrentFlowInfo::CompleteAuthorize { .. }),
                    Some(enums::PaymentMethodType::PixAutomaticoQr),
                ) => {
                    // redundant case
                    Some(AccessTokenUrlPath::Leg2)
                }
                // SetupMandate flow
                (
                    Some(CurrentFlowInfo::SetupMandate { .. }),
                    Some(enums::PaymentMethodType::Boleto),
                ) => Some(AccessTokenUrlPath::Boleto),
                (
                    Some(CurrentFlowInfo::SetupMandate { .. }),
                    Some(enums::PaymentMethodType::Pix),
                ) => Some(AccessTokenUrlPath::Leg1),
                (
                    Some(CurrentFlowInfo::SetupMandate { .. }),
                    Some(enums::PaymentMethodType::PixAutomaticoPush),
                ) => Some(AccessTokenUrlPath::Leg2),
                (
                    Some(CurrentFlowInfo::SetupMandate { .. }),
                    Some(enums::PaymentMethodType::PixAutomaticoQr),
                ) => Some(AccessTokenUrlPath::Leg2),

                (None, Some(enums::PaymentMethodType::Boleto)) => Some(AccessTokenUrlPath::Boleto),
                (None, Some(enums::PaymentMethodType::Pix)) => Some(AccessTokenUrlPath::Leg1),
                (
                    Some(CurrentFlowInfo::Psync { .. }),
                    Some(enums::PaymentMethodType::PixAutomaticoPush),
                ) => Some(AccessTokenUrlPath::Leg2),
                (
                    Some(CurrentFlowInfo::Psync { request_data }),
                    Some(enums::PaymentMethodType::PixAutomaticoQr),
                ) => {
                    let expiry_type = get_qr_code_type(request_data.connector_meta);
                    match expiry_type {
                        Some(ExpiryType::Immediate) | Some(ExpiryType::Scheduled) => {
                            Some(AccessTokenUrlPath::Leg1)
                        }
                        None => Some(AccessTokenUrlPath::Leg2),
                    }
                }
                (None, Some(enums::PaymentMethodType::PixAutomaticoPush)) => {
                    Some(AccessTokenUrlPath::Leg2)
                }
                (None, Some(enums::PaymentMethodType::PixAutomaticoQr)) => None,
                // No payment method type or unsupported payment method type
                (_, None) => None,
                (_, Some(_)) => None,
            }
        }
    }
}

impl From<SantanderJourneyType> for Option<ExpiryType> {
    fn from(item: SantanderJourneyType) -> Self {
        match item {
            SantanderJourneyType::Jornada3 => Some(ExpiryType::Immediate),
            SantanderJourneyType::Jornada4 => Some(ExpiryType::Scheduled),
            _ => None,
        }
    }
}
