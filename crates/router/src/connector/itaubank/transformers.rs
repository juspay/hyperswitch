use api_models::payments;
use common_utils::{ext_traits::Encode, types::StringMajorUnit};
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;

use crate::{
    connector::utils::{self, RouterData},
    core::errors,
    types::{self, api, domain, storage::enums},
    utils as crate_utils,
};

pub struct ItaubankRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for ItaubankRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct ItaubankPaymentsRequest {
    valor: PixPaymentValue,  // amount
    chave: Secret<String>,   // pix-key
    devedor: ItaubankDebtor, // debtor
}

#[derive(Default, Debug, Serialize)]
pub struct PixPaymentValue {
    original: StringMajorUnit,
}

#[derive(Default, Debug, Serialize)]
pub struct ItaubankDebtor {
    #[serde(skip_serializing_if = "Option::is_none")]
    cpf: Option<Secret<String>>, // CPF is a Brazilian tax identification number
    #[serde(skip_serializing_if = "Option::is_none")]
    cnpj: Option<Secret<String>>, // CNPJ is a Brazilian company tax identification number
    #[serde(skip_serializing_if = "Option::is_none")]
    nome: Option<Secret<String>>, // name of the debtor
}

impl TryFrom<&ItaubankRouterData<&types::PaymentsAuthorizeRouterData>> for ItaubankPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ItaubankRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            domain::PaymentMethodData::BankTransfer(bank_transfer_data) => {
                match *bank_transfer_data {
                    domain::BankTransferData::Pix { pix_key, cpf, cnpj } => {
                        let nome = item.router_data.get_optional_billing_full_name();
                        // cpf and cnpj are mutually exclusive
                        let devedor = match (cpf, cnpj) {
                            (Some(cpf), _) => ItaubankDebtor {
                                cpf: Some(cpf),
                                cnpj: None,
                                nome,
                            },
                            (None, Some(cnpj)) => ItaubankDebtor {
                                cpf: None,
                                cnpj: Some(cnpj),
                                nome,
                            },
                            _ => Err(errors::ConnectorError::MissingRequiredField {
                                field_name: "cpf and cnpj both missing in payment_method_data",
                            })?,
                        };
                        Ok(Self {
                            valor: PixPaymentValue {
                                original: item.amount.to_owned(),
                            },
                            chave: pix_key.ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "pix_key",
                            })?,
                            devedor,
                        })
                    }
                    _ => Err(
                        errors::ConnectorError::NotImplemented("Payment methods".to_string())
                            .into(),
                    ),
                }
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

pub struct ItaubankAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for ItaubankAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                client_secret: api_key.to_owned(),
                client_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ItaubankAuthRequest {
    client_id: Secret<String>,
    client_secret: Secret<String>,
    grant_type: ItaubankGrantType,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItaubankGrantType {
    ClientCredentials,
}

impl TryFrom<&types::RefreshTokenRouterData> for ItaubankAuthRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth_details = ItaubankAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            client_id: auth_details.client_id,
            client_secret: auth_details.client_secret,
            grant_type: ItaubankGrantType::ClientCredentials,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItaubankUpdateTokenResponse {
    access_token: Secret<String>,
    expires_in: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItaubankTokenErrorResponse {
    pub status: i64,
    pub title: Option<String>,
    pub detail: Option<String>,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, ItaubankUpdateTokenResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, ItaubankUpdateTokenResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ItaubankPaymentStatus {
    Ativa,                        // Active
    Concluida,                    // Completed
    RemovidaPeloPsp,              // Removed by PSP
    RemovidaPeloUsuarioRecebedor, // Removed by receiving User
}

impl From<ItaubankPaymentStatus> for enums::AttemptStatus {
    fn from(item: ItaubankPaymentStatus) -> Self {
        match item {
            ItaubankPaymentStatus::Ativa => Self::AuthenticationPending,
            ItaubankPaymentStatus::Concluida => Self::Charged,
            ItaubankPaymentStatus::RemovidaPeloPsp
            | ItaubankPaymentStatus::RemovidaPeloUsuarioRecebedor => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItaubankPaymentsResponse {
    status: ItaubankPaymentStatus,
    calendario: ItaubankPixExpireTime,
    txid: String,
    #[serde(rename = "pixCopiaECola")]
    pix_qr_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItaubankPixExpireTime {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    criacao: PrimitiveDateTime,
    expiracao: i64,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, ItaubankPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            ItaubankPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let connector_metadata = get_qr_code_data(&item.response)?;
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.txid.to_owned(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.txid),
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

fn get_qr_code_data(
    response: &ItaubankPaymentsResponse,
) -> errors::CustomResult<Option<serde_json::Value>, errors::ConnectorError> {
    let creation_time = utils::get_timestamp_in_milliseconds(&response.calendario.criacao);
    // convert expiration to milliseconds and add to creation time
    let expiration_time = creation_time + (response.calendario.expiracao * 1000);

    let image_data = crate_utils::QrImage::new_from_data(response.pix_qr_value.clone())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let image_data_url = Url::parse(image_data.data.clone().as_str())
        .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

    let qr_code_info = payments::QrCodeInformation::QrDataUrl {
        image_data_url,
        display_to_timestamp: Some(expiration_time),
    };

    Some(qr_code_info.encode_to_value())
        .transpose()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItaubankPaymentsSyncResponse {
    status: ItaubankPaymentStatus,
    txid: String,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, ItaubankPaymentsSyncResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            ItaubankPaymentsSyncResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.txid.to_owned(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.txid),
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct ItaubankRefundRequest {
    pub amount: StringMajorUnit,
}

impl<F> TryFrom<&ItaubankRouterData<&types::RefundsRouterData<F>>> for ItaubankRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ItaubankRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItaubankErrorResponse {
    pub error: ItaubankErrorBody,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItaubankErrorBody {
    pub status: u16,
    pub title: Option<String>,
    pub detail: Option<String>,
}
