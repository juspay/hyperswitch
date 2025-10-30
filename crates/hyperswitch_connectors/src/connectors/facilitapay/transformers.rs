use api_models::payments::QrCodeInformation;
use common_enums::{enums, PaymentMethod};
use common_utils::{
    errors::CustomResult,
    ext_traits::{BytesExt, Encode},
    new_type::MaskedBankAccount,
    pii,
    types::StringMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankTransferData, PaymentMethodData},
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{
        ConnectorCustomerResponseData, PaymentsResponseData, RefundsResponseData,
    },
    types,
};
use hyperswitch_interfaces::{
    consts, errors, events::connector_api_logs::ConnectorEvent, types::Response,
};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;

use super::{
    requests::{
        DocumentType, FacilitapayAuthRequest, FacilitapayCredentials, FacilitapayCustomerRequest,
        FacilitapayPaymentsRequest, FacilitapayPerson, FacilitapayRouterData,
        FacilitapayTransactionRequest, PixTransactionRequest,
    },
    responses::{
        FacilitapayAuthResponse, FacilitapayCustomerResponse, FacilitapayPaymentStatus,
        FacilitapayPaymentsResponse, FacilitapayRefundResponse, FacilitapayVoidResponse,
    },
};
use crate::{
    types::{
        PaymentsCancelResponseRouterData, RefreshTokenRouterData, RefundsResponseRouterData,
        ResponseRouterData,
    },
    utils::{self, is_payment_failure, missing_field_err, QrImage, RouterData as OtherRouterData},
};
type Error = error_stack::Report<errors::ConnectorError>;

impl<T> From<(StringMajorUnit, T)> for FacilitapayRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// Auth Struct
#[derive(Debug, Clone)]
pub struct FacilitapayAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacilitapayConnectorMetadataObject {
    // pub destination_account_number: Secret<String>,
    pub destination_account_number: MaskedBankAccount,
}

// Helper to build the request from Hyperswitch Auth Type
impl FacilitapayAuthRequest {
    fn from_auth_type(auth: &FacilitapayAuthType) -> Self {
        Self {
            user: FacilitapayCredentials {
                username: auth.username.clone(),
                password: auth.password.clone(),
            },
        }
    }
}

impl TryFrom<&ConnectorAuthType> for FacilitapayAuthType {
    type Error = Error;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                username: key1.to_owned(),
                password: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl TryFrom<&RefreshTokenRouterData> for FacilitapayAuthRequest {
    type Error = Error;
    fn try_from(item: &RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth_type = FacilitapayAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self::from_auth_type(&auth_type))
    }
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for FacilitapayConnectorMetadataObject {
    type Error = Error;

    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "merchant_connector_account.metadata",
            })?;

        Ok(metadata)
    }
}

impl TryFrom<&FacilitapayRouterData<&types::PaymentsAuthorizeRouterData>>
    for FacilitapayPaymentsRequest
{
    type Error = Error;
    fn try_from(
        item: &FacilitapayRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let metadata =
            FacilitapayConnectorMetadataObject::try_from(&item.router_data.connector_meta_data)?;

        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankTransfer(bank_transfer_data) => match *bank_transfer_data {
                BankTransferData::Pix {
                    source_bank_account_id,
                    ..
                } => {
                    // Set expiry time to 15 minutes from now
                    let dynamic_pix_expires_at = {
                        let now = time::OffsetDateTime::now_utc();
                        let expires_at = now + time::Duration::minutes(15);

                        PrimitiveDateTime::new(expires_at.date(), expires_at.time())
                    };

                    let transaction_data =
                        FacilitapayTransactionRequest::Pix(PixTransactionRequest {
                            // subject id must be generated by pre-process step and link with customer id
                            // might require discussions to be done
                            subject_id: item.router_data.get_connector_customer_id()?.into(),
                            from_bank_account_id: source_bank_account_id.clone().ok_or(
                                errors::ConnectorError::MissingRequiredField {
                                    field_name: "source bank account id",
                                },
                            )?,

                            to_bank_account_id: metadata.destination_account_number,
                            currency: item.router_data.request.currency,
                            exchange_currency: item.router_data.request.currency,
                            value: item.amount.clone(),
                            use_dynamic_pix: true,
                            // Format: YYYY-MM-DDThh:mm:ssZ
                            dynamic_pix_expires_at,
                        });

                    Ok(Self {
                        transaction: transaction_data,
                    })
                }
                BankTransferData::AchBankTransfer {}
                | BankTransferData::SepaBankTransfer {}
                | BankTransferData::BacsBankTransfer {}
                | BankTransferData::MultibancoBankTransfer {}
                | BankTransferData::PermataBankTransfer {}
                | BankTransferData::BcaBankTransfer {}
                | BankTransferData::BniVaBankTransfer {}
                | BankTransferData::BriVaBankTransfer {}
                | BankTransferData::CimbVaBankTransfer {}
                | BankTransferData::DanamonVaBankTransfer {}
                | BankTransferData::MandiriVaBankTransfer {}
                | BankTransferData::Pse {}
                | BankTransferData::InstantBankTransfer {}
                | BankTransferData::InstantBankTransferFinland {}
                | BankTransferData::InstantBankTransferPoland {}
                | BankTransferData::IndonesianBankTransfer { .. }
                | BankTransferData::LocalBankTransfer { .. } => {
                    Err(errors::ConnectorError::NotImplemented(
                        "Selected payment method through Facilitapay".to_string(),
                    )
                    .into())
                }
            },
            PaymentMethodData::Card(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    "Selected payment method through Facilitapay".to_string(),
                )
                .into())
            }
        }
    }
}

fn convert_to_document_type(document_type: &str) -> Result<DocumentType, errors::ConnectorError> {
    match document_type.to_lowercase().as_str() {
        "cc" => Ok(DocumentType::CedulaDeCiudadania),
        "cnpj" => Ok(DocumentType::CadastroNacionaldaPessoaJurídica),
        "cpf" => Ok(DocumentType::CadastrodePessoasFísicas),
        "curp" => Ok(DocumentType::ClaveÚnicadeRegistrodePoblación),
        "nit" => Ok(DocumentType::NúmerodeIdentificaciónTributaria),
        "passport" => Ok(DocumentType::Passport),
        "rfc" => Ok(DocumentType::RegistroFederaldeContribuyentes),
        "rut" => Ok(DocumentType::RolUnicoTributario),
        "tax_id" | "taxid" => Ok(DocumentType::TaxId),
        _ => Err(errors::ConnectorError::NotSupported {
            message: format!("Document type '{document_type}'"),
            connector: "Facilitapay",
        }),
    }
}

pub fn parse_facilitapay_error_response(
    res: Response,
    event_builder: Option<&mut ConnectorEvent>,
) -> CustomResult<ErrorResponse, errors::ConnectorError> {
    let status_code = res.status_code;
    let response_body_bytes = res.response.clone();

    let (message, raw_error) =
        match response_body_bytes.parse_struct::<serde_json::Value>("FacilitapayErrorResponse") {
            Ok(json_value) => {
                event_builder.map(|i| i.set_response_body(&json_value));

                let message = extract_message_from_json(&json_value);
                (
                    message,
                    serde_json::to_string(&json_value).unwrap_or_default(),
                )
            }
            Err(_) => match String::from_utf8(response_body_bytes.to_vec()) {
                Ok(text) => {
                    event_builder.map(|i| i.set_response_body(&text));
                    (text.clone(), text)
                }
                Err(_) => (
                    "Invalid response format received".to_string(),
                    format!(
                    "Unable to parse response as JSON or UTF-8 string. Status code: {status_code}",
                ),
                ),
            },
        };

    Ok(ErrorResponse {
        status_code,
        code: consts::NO_ERROR_CODE.to_string(),
        message,
        reason: Some(raw_error),
        attempt_status: None,
        connector_transaction_id: None,
        network_advice_code: None,
        network_decline_code: None,
        network_error_message: None,
        connector_metadata: None,
    })
}

// Helper function to extract a readable message from JSON error
fn extract_message_from_json(json: &serde_json::Value) -> String {
    if let Some(obj) = json.as_object() {
        if let Some(error) = obj.get("error").and_then(|e| e.as_str()) {
            return error.to_string();
        }

        if obj.contains_key("errors") {
            return "Validation error occurred".to_string();
        }

        if !obj.is_empty() {
            return obj
                .iter()
                .next()
                .map(|(k, v)| format!("{k}: {v}"))
                .unwrap_or_else(|| "Unknown error".to_string());
        }
    } else if let Some(s) = json.as_str() {
        return s.to_string();
    }

    "Unknown error format".to_string()
}

impl TryFrom<&types::ConnectorCustomerRouterData> for FacilitapayCustomerRequest {
    type Error = Error;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        let email = item.request.email.clone();

        let social_name = item.get_billing_full_name()?;

        let (document_type, document_number) = match item.request.payment_method_data.clone() {
            Some(PaymentMethodData::BankTransfer(bank_transfer_data)) => {
                match *bank_transfer_data {
                    BankTransferData::Pix { cpf, .. } => {
                        // Extract only digits from the CPF string
                        let document_number =
                            cpf.ok_or_else(missing_field_err("cpf"))?.map(|cpf_number| {
                                cpf_number
                                    .chars()
                                    .filter(|chars| chars.is_ascii_digit())
                                    .collect::<String>()
                            });

                        let document_type = convert_to_document_type("cpf")?;
                        (document_type, document_number)
                    }
                    _ => {
                        return Err(errors::ConnectorError::NotImplemented(
                            "Selected payment method through Facilitapay".to_string(),
                        )
                        .into())
                    }
                }
            }
            _ => {
                return Err(errors::ConnectorError::NotImplemented(
                    "Selected payment method through Facilitapay".to_string(),
                )
                .into())
            }
        };

        let fiscal_country = item.get_billing_country()?;

        let person = FacilitapayPerson {
            document_number,
            document_type,
            social_name,
            fiscal_country,
            email,
            birth_date: None,
            phone_country_code: None,
            phone_area_code: None,
            phone_number: None,
            address_city: None,
            address_state: None,
            address_complement: None,
            address_country: None,
            address_number: None,
            address_postal_code: None,
            address_street: None,
            net_monthly_average_income: None,
        };

        Ok(Self { person })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, FacilitapayCustomerResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, FacilitapayCustomerResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new_with_customer_id(
                    item.response.data.customer_id.expose(),
                ),
            )),
            ..item.data
        })
    }
}

impl From<FacilitapayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: FacilitapayPaymentStatus) -> Self {
        match item {
            FacilitapayPaymentStatus::Pending => Self::Pending,
            FacilitapayPaymentStatus::Identified
            | FacilitapayPaymentStatus::Exchanged
            | FacilitapayPaymentStatus::Wired => Self::Charged,
            FacilitapayPaymentStatus::Cancelled => Self::Failure,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, FacilitapayAuthResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, FacilitapayAuthResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.jwt,
                expires: 86400, // Facilitapay docs say 24 hours validity. 24 * 60 * 60 = 86400 seconds.
            }),
            ..item.data
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, FacilitapayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, FacilitapayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = if item.data.payment_method == PaymentMethod::BankTransfer
            && item.response.data.status == FacilitapayPaymentStatus::Identified
        {
            if item.response.data.currency != item.response.data.exchange_currency {
                // Cross-currency: Identified is not terminal
                common_enums::AttemptStatus::Pending
            } else {
                // Local currency: Identified is terminal
                common_enums::AttemptStatus::Charged
            }
        } else {
            common_enums::AttemptStatus::from(item.response.data.status.clone())
        };

        Ok(Self {
            status,
            response: if is_payment_failure(status) {
                Err(ErrorResponse {
                    code: item.response.data.status.clone().to_string(),
                    message: item.response.data.status.clone().to_string(),
                    reason: item.response.data.cancelled_reason,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.data.transaction_id),
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            } else {
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.response.data.transaction_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: get_qr_code_data(&item.response)?,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.data.transaction_id),
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            },
            ..item.data
        })
    }
}

fn get_qr_code_data(
    response: &FacilitapayPaymentsResponse,
) -> CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let expiration_time: i64 = if let Some(meta) = &response.data.meta {
        if let Some(due_date_str) = meta
            .get("dynamic_pix_due_date")
            .and_then(|due_date_value| due_date_value.as_str())
        {
            let datetime = time::OffsetDateTime::parse(
                due_date_str,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?;

            datetime.unix_timestamp() * 1000
        } else {
            // If dynamic_pix_due_date isn't present, use current time + 15 minutes
            let now = time::OffsetDateTime::now_utc();
            let expires_at = now + time::Duration::minutes(15);
            expires_at.unix_timestamp() * 1000
        }
    } else {
        // If meta is null, use current time + 15 minutes
        let now = time::OffsetDateTime::now_utc();
        let expires_at = now + time::Duration::minutes(15);
        expires_at.unix_timestamp() * 1000
    };

    let dynamic_pix_code = response.data.dynamic_pix_code.as_ref().ok_or_else(|| {
        errors::ConnectorError::MissingRequiredField {
            field_name: "dynamic_pix_code",
        }
    })?;

    let image_data = QrImage::new_from_data(dynamic_pix_code.clone())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let image_data_url = Url::parse(image_data.data.clone().as_str())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let qr_code_info = QrCodeInformation::QrDataUrl {
        image_data_url,
        display_to_timestamp: Some(expiration_time),
    };

    Some(qr_code_info.encode_to_value())
        .transpose()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

impl From<FacilitapayPaymentStatus> for enums::RefundStatus {
    fn from(item: FacilitapayPaymentStatus) -> Self {
        match item {
            FacilitapayPaymentStatus::Identified
            | FacilitapayPaymentStatus::Exchanged
            | FacilitapayPaymentStatus::Wired => Self::Success,
            FacilitapayPaymentStatus::Cancelled => Self::Failure,
            FacilitapayPaymentStatus::Pending => Self::Pending,
        }
    }
}

// Void (cancel unprocessed payment) transformer
impl TryFrom<PaymentsCancelResponseRouterData<FacilitapayVoidResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = Error;
    fn try_from(
        item: PaymentsCancelResponseRouterData<FacilitapayVoidResponse>,
    ) -> Result<Self, Self::Error> {
        let status = common_enums::AttemptStatus::from(item.response.data.status.clone());

        Ok(Self {
            status,
            response: if is_payment_failure(status) {
                Err(ErrorResponse {
                    code: item.response.data.status.clone().to_string(),
                    message: item.response.data.status.clone().to_string(),
                    reason: item.response.data.reason,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.data.void_id.clone()),
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                })
            } else {
                Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.response.data.void_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.data.void_id),
                    incremental_authorization_allowed: None,
                    charges: None,
                })
            },
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, FacilitapayRefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<Execute, FacilitapayRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.data.transaction_id.clone(),
                refund_status: enums::RefundStatus::from(item.response.data.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, FacilitapayRefundResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = Error;
    fn try_from(
        item: RefundsResponseRouterData<RSync, FacilitapayRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.data.transaction_id.clone(),
                refund_status: enums::RefundStatus::from(item.response.data.status),
            }),
            ..item.data
        })
    }
}
