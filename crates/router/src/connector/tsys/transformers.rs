use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{CardData, PaymentsAuthorizeRequestData, RefundsRequestData},
    core::errors,
    types::{
        self, api,
        storage::{self, enums},
    },
};

#[derive(Debug, Serialize)]
pub enum TsysPaymentsRequest {
    Auth(TsysPaymentAuthSaleRequest),
    Sale(TsysPaymentAuthSaleRequest),
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysPaymentAuthSaleRequest {
    #[serde(rename = "deviceID")]
    device_id: Secret<String>,
    transaction_key: Secret<String>,
    card_data_source: String,
    transaction_amount: String,
    currency_code: storage::enums::Currency,
    card_number: cards::CardNumber,
    expiration_date: Secret<String>,
    cvv2: Secret<String>,
    terminal_capability: String,
    terminal_operating_environment: String,
    cardholder_authentication_method: String,
    #[serde(rename = "developerID")]
    developer_id: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for TsysPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => {
                let connector_auth: TsysAuthType =
                    TsysAuthType::try_from(&item.connector_auth_type)?;
                let auth_data: TsysPaymentAuthSaleRequest = TsysPaymentAuthSaleRequest {
                    device_id: connector_auth.device_id,
                    transaction_key: connector_auth.transaction_key,
                    card_data_source: "INTERNET".to_string(),
                    transaction_amount: item.request.amount.to_string(),
                    currency_code: item.request.currency,
                    card_number: ccard.card_number.clone(),
                    expiration_date: ccard
                        .get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned()),
                    cvv2: ccard.card_cvc,
                    terminal_capability: "ICC_CHIP_READ_ONLY".to_string(),
                    terminal_operating_environment: "ON_MERCHANT_PREMISES_ATTENDED".to_string(),
                    cardholder_authentication_method: "NOT_AUTHENTICATED".to_string(),
                    developer_id: connector_auth.developer_id,
                };
                if item.request.is_auto_capture()? {
                    Ok(Self::Sale(auth_data))
                } else {
                    Ok(Self::Auth(auth_data))
                }
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct TsysAuthType {
    pub(super) device_id: Secret<String>,
    pub(super) transaction_key: Secret<String>,
    pub(super) developer_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for TsysAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                device_id: Secret::new(api_key.to_string()),
                transaction_key: Secret::new(key1.to_string()),
                developer_id: Secret::new(api_secret.to_string()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TsysPaymentStatus {
    Pass,
    Fail,
}

impl From<TsysPaymentStatus> for enums::AttemptStatus {
    fn from(item: TsysPaymentStatus) -> Self {
        match item {
            TsysPaymentStatus::Pass => Self::Charged,
            TsysPaymentStatus::Fail => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(clippy::enum_variant_names)]
pub enum TsysPaymentsResponse {
    AuthResponse(TsysResponse),
    SaleResponse(TsysResponse),
    CaptureResponse(TsysResponse),
    SearchTransactionResponse(TsysResponse),
    VoidResponse(TsysResponse),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysResponse {
    pub status: TsysPaymentStatus,
    pub response_code: String,
    pub response_message: String,
    #[serde(rename = "transactionID")]
    pub transaction_id: Option<String>,
    pub transaction_amount: Option<String>,
    pub transaction_details: Option<TsysTransactionDetails>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysTransactionDetails {
    #[serde(rename = "transactionID")]
    transaction_id: String,
    transaction_type: String,
    transaction_status: TsysTransactionStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TsysTransactionStatus {
    Approved,
    Declined,
    Void,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, TsysPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, TsysPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (transaction_id, status, response_code, response_message, amount_captured) = match item
            .response
        {
            TsysPaymentsResponse::AuthResponse(auth_response) => (
                auth_response.transaction_id,
                match auth_response.status {
                    TsysPaymentStatus::Pass => enums::AttemptStatus::Authorized,
                    TsysPaymentStatus::Fail => enums::AttemptStatus::AuthorizationFailed,
                },
                auth_response.response_code,
                auth_response.response_message,
                None,
            ),
            TsysPaymentsResponse::SaleResponse(sale_response) => (
                sale_response.transaction_id,
                enums::AttemptStatus::from(sale_response.status),
                sale_response.response_code,
                sale_response.response_message,
                None,
            ),
            TsysPaymentsResponse::CaptureResponse(capture_response) => (
                capture_response.transaction_id,
                enums::AttemptStatus::from(capture_response.status),
                capture_response.response_code,
                capture_response.response_message,
                capture_response
                    .transaction_amount
                    .map(|amount| amount.parse::<i64>())
                    .transpose()
                    .into_report()
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?,
            ),
            TsysPaymentsResponse::SearchTransactionResponse(search_response) => {
                let (status, transaction_id) = search_response.transaction_details.map_or(
                    (enums::AttemptStatus::Pending, None),
                    |transaction_details| {
                        (
                            match transaction_details.transaction_status {
                                TsysTransactionStatus::Approved => {
                                    if transaction_details.transaction_type.contains("Auth-Only") {
                                        enums::AttemptStatus::Authorized
                                    } else {
                                        enums::AttemptStatus::Charged
                                    }
                                }
                                TsysTransactionStatus::Void => enums::AttemptStatus::Voided,
                                TsysTransactionStatus::Declined => enums::AttemptStatus::Failure,
                            },
                            Some(transaction_details.transaction_id),
                        )
                    },
                );
                (
                    transaction_id,
                    status,
                    search_response.response_code,
                    search_response.response_message,
                    search_response
                        .transaction_amount
                        .map(|amount| amount.parse::<i64>())
                        .transpose()
                        .into_report()
                        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?,
                )
            }
            TsysPaymentsResponse::VoidResponse(void_response) => (
                void_response.transaction_id,
                match void_response.status {
                    TsysPaymentStatus::Pass => enums::AttemptStatus::Voided,
                    TsysPaymentStatus::Fail => enums::AttemptStatus::VoidFailed,
                },
                void_response.response_code,
                void_response.response_message,
                None,
            ),
        };
        let response = if response_code.chars().next().is_some_and(|x| x == 'A') {
            Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: transaction_id.map_or(types::ResponseId::NoResponseId, |t| {
                    types::ResponseId::ConnectorTransactionId(t)
                }),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            })
        } else {
            Err(types::ErrorResponse {
                code: response_code,
                message: response_message.clone(),
                reason: Some(response_message),
                status_code: item.http_code,
            })
        };
        Ok(Self {
            status,
            response,
            amount_captured,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysSearchTransactionRequest {
    #[serde(rename = "deviceID")]
    device_id: Secret<String>,
    transaction_key: Secret<String>,
    #[serde(rename = "transactionID")]
    transaction_id: String,
    #[serde(rename = "developerID")]
    developer_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TsysSyncRequest {
    search_transaction: TsysSearchTransactionRequest,
}

impl TryFrom<&types::PaymentsSyncRouterData> for TsysSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let connector_auth: TsysAuthType = TsysAuthType::try_from(&item.connector_auth_type)?;
        let search_transaction = TsysSearchTransactionRequest {
            device_id: connector_auth.device_id,
            transaction_key: connector_auth.transaction_key,
            transaction_id: item
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
            developer_id: connector_auth.developer_id,
        };
        Ok(Self { search_transaction })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysCancelRequest {
    #[serde(rename = "deviceID")]
    device_id: Secret<String>,
    transaction_key: Secret<String>,
    #[serde(rename = "transactionID")]
    transaction_id: String,
    #[serde(rename = "developerID")]
    developer_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TsysPaymentsCancelRequest {
    void: TsysCancelRequest,
}
impl TryFrom<&types::PaymentsCancelRouterData> for TsysPaymentsCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let connector_auth: TsysAuthType = TsysAuthType::try_from(&item.connector_auth_type)?;
        let void = TsysCancelRequest {
            device_id: connector_auth.device_id,
            transaction_key: connector_auth.transaction_key,
            transaction_id: item.request.connector_transaction_id.clone(),
            developer_id: connector_auth.developer_id,
        };
        Ok(Self { void })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysCaptureRequest {
    #[serde(rename = "deviceID")]
    device_id: Secret<String>,
    transaction_key: Secret<String>,
    transaction_amount: String,
    #[serde(rename = "transactionID")]
    transaction_id: String,
    #[serde(rename = "developerID")]
    developer_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]

pub struct TsysPaymentsCaptureRequest {
    capture: TsysCaptureRequest,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for TsysPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let connector_auth: TsysAuthType = TsysAuthType::try_from(&item.connector_auth_type)?;
        let capture = TsysCaptureRequest {
            device_id: connector_auth.device_id,
            transaction_key: connector_auth.transaction_key,
            transaction_id: item.request.connector_transaction_id.clone(),
            developer_id: connector_auth.developer_id,
            transaction_amount: item.request.amount_to_capture.to_string(),
        };
        Ok(Self { capture })
    }
}
// REFUND :
// Type definition for RefundRequest

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysReturnRequest {
    #[serde(rename = "deviceID")]
    device_id: Secret<String>,
    transaction_key: Secret<String>,
    transaction_amount: String,
    #[serde(rename = "transactionID")]
    transaction_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TsysRefundRequest {
    #[serde(rename = "Return")]
    return_request: TsysReturnRequest,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for TsysRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let connector_auth: TsysAuthType = TsysAuthType::try_from(&item.connector_auth_type)?;
        let return_request = TsysReturnRequest {
            device_id: connector_auth.device_id,
            transaction_key: connector_auth.transaction_key,
            transaction_amount: item.request.refund_amount.to_string(),
            transaction_id: item.request.connector_transaction_id.clone(),
        };
        Ok(Self { return_request })
    }
}

impl From<TsysPaymentStatus> for enums::RefundStatus {
    fn from(item: TsysPaymentStatus) -> Self {
        match item {
            TsysPaymentStatus::Pass => Self::Success,
            TsysPaymentStatus::Fail => Self::Failure,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefundResponse {
    return_response: TsysResponse,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let return_response = item.response.return_response;
        let response = if return_response
            .response_code
            .chars()
            .next()
            .is_some_and(|x| x == 'A')
        {
            Ok(types::RefundsResponseData {
                connector_refund_id: return_response.transaction_id.unwrap_or_default(),
                refund_status: enums::RefundStatus::from(return_response.status),
            })
        } else {
            Err(types::ErrorResponse {
                code: return_response.response_code,
                message: return_response.response_message.clone(),
                reason: Some(return_response.response_message),
                status_code: item.http_code,
            })
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl TryFrom<&types::RefundSyncRouterData> for TsysSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let connector_auth: TsysAuthType = TsysAuthType::try_from(&item.connector_auth_type)?;
        let search_transaction = TsysSearchTransactionRequest {
            device_id: connector_auth.device_id,
            transaction_key: connector_auth.transaction_key,
            transaction_id: item.request.get_connector_refund_id()?,
            developer_id: connector_auth.developer_id,
        };
        Ok(Self { search_transaction })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefundSyncResponse {
    search_transaction_response: TsysResponse,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let search_response = item.response.search_transaction_response;
        let (refund_status, transaction_id) = search_response.transaction_details.map_or(
            (enums::RefundStatus::Pending, None),
            |transaction_details| {
                (
                    match transaction_details.transaction_status {
                        TsysTransactionStatus::Void => enums::RefundStatus::Success,
                        TsysTransactionStatus::Declined => enums::RefundStatus::Failure,
                        TsysTransactionStatus::Approved => enums::RefundStatus::Pending,
                    },
                    Some(transaction_details.transaction_id),
                )
            },
        );
        let response = if search_response
            .response_code
            .chars()
            .next()
            .is_some_and(|x| x == 'A')
        {
            Ok(types::RefundsResponseData {
                connector_refund_id: transaction_id.unwrap_or_default(),
                refund_status,
            })
        } else {
            Err(types::ErrorResponse {
                code: search_response.response_code,
                message: search_response.response_message.clone(),
                reason: Some(search_response.response_message),
                status_code: item.http_code,
            })
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}
