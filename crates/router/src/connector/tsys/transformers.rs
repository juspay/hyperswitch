use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, CardData, PaymentsAuthorizeRequestData, RefundsRequestData},
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
    order_number: String,
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
                    order_number: item.connector_request_reference_id.clone(),
                };
                if item.request.is_auto_capture()? {
                    Ok(Self::Sale(auth_data))
                } else {
                    Ok(Self::Auth(auth_data))
                }
            }
            api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Wallet(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("tsys"),
            ))?,
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
                device_id: api_key.to_owned(),
                transaction_key: key1.to_owned(),
                developer_id: api_secret.to_owned(),
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TsysTransactionStatus {
    Approved,
    Declined,
    Void,
}

impl From<TsysTransactionDetails> for enums::AttemptStatus {
    fn from(item: TsysTransactionDetails) -> Self {
        match item.transaction_status {
            TsysTransactionStatus::Approved => {
                if item.transaction_type.contains("Auth-Only") {
                    Self::Authorized
                } else {
                    Self::Charged
                }
            }
            TsysTransactionStatus::Void => Self::Voided,
            TsysTransactionStatus::Declined => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysErrorResponse {
    pub status: TsysPaymentStatus,
    pub response_code: String,
    pub response_message: String,
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
#[serde(rename_all = "camelCase")]
pub struct TsysPaymentsSyncResponse {
    pub status: TsysPaymentStatus,
    pub response_code: String,
    pub response_message: String,
    pub transaction_details: TsysTransactionDetails,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysResponse {
    pub status: TsysPaymentStatus,
    pub response_code: String,
    pub response_message: String,
    #[serde(rename = "transactionID")]
    pub transaction_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TsysResponseTypes {
    SuccessResponse(TsysResponse),
    ErrorResponse(TsysErrorResponse),
}

#[derive(Debug, Clone, Deserialize)]
#[allow(clippy::enum_variant_names)]
pub enum TsysPaymentsResponse {
    AuthResponse(TsysResponseTypes),
    SaleResponse(TsysResponseTypes),
    CaptureResponse(TsysResponseTypes),
    VoidResponse(TsysResponseTypes),
}

fn get_error_response(
    connector_error_response: TsysErrorResponse,
    status_code: u16,
) -> types::ErrorResponse {
    types::ErrorResponse {
        code: connector_error_response.response_code,
        message: connector_error_response.response_message.clone(),
        reason: Some(connector_error_response.response_message),
        status_code,
    }
}

fn get_payments_response(connector_response: TsysResponse) -> types::PaymentsResponseData {
    types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::ConnectorTransactionId(
            connector_response.transaction_id.clone(),
        ),
        redirection_data: None,
        mandate_reference: None,
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: Some(connector_response.transaction_id),
    }
}

fn get_payments_sync_response(
    connector_response: &TsysPaymentsSyncResponse,
) -> types::PaymentsResponseData {
    types::PaymentsResponseData::TransactionResponse {
        resource_id: types::ResponseId::ConnectorTransactionId(
            connector_response
                .transaction_details
                .transaction_id
                .clone(),
        ),
        redirection_data: None,
        mandate_reference: None,
        connector_metadata: None,
        network_txn_id: None,
        connector_response_reference_id: Some(
            connector_response
                .transaction_details
                .transaction_id
                .clone(),
        ),
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, TsysPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, TsysPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response {
            TsysPaymentsResponse::AuthResponse(resp) => match resp {
                TsysResponseTypes::SuccessResponse(auth_response) => (
                    Ok(get_payments_response(auth_response)),
                    enums::AttemptStatus::Authorized,
                ),
                TsysResponseTypes::ErrorResponse(connector_error_response) => (
                    Err(get_error_response(connector_error_response, item.http_code)),
                    enums::AttemptStatus::AuthorizationFailed,
                ),
            },
            TsysPaymentsResponse::SaleResponse(resp) => match resp {
                TsysResponseTypes::SuccessResponse(sale_response) => (
                    Ok(get_payments_response(sale_response)),
                    enums::AttemptStatus::Charged,
                ),
                TsysResponseTypes::ErrorResponse(connector_error_response) => (
                    Err(get_error_response(connector_error_response, item.http_code)),
                    enums::AttemptStatus::Failure,
                ),
            },
            TsysPaymentsResponse::CaptureResponse(resp) => match resp {
                TsysResponseTypes::SuccessResponse(capture_response) => (
                    Ok(get_payments_response(capture_response)),
                    enums::AttemptStatus::Charged,
                ),
                TsysResponseTypes::ErrorResponse(connector_error_response) => (
                    Err(get_error_response(connector_error_response, item.http_code)),
                    enums::AttemptStatus::CaptureFailed,
                ),
            },
            TsysPaymentsResponse::VoidResponse(resp) => match resp {
                TsysResponseTypes::SuccessResponse(void_response) => (
                    Ok(get_payments_response(void_response)),
                    enums::AttemptStatus::Voided,
                ),
                TsysResponseTypes::ErrorResponse(connector_error_response) => (
                    Err(get_error_response(connector_error_response, item.http_code)),
                    enums::AttemptStatus::VoidFailed,
                ),
            },
        };
        Ok(Self {
            status,
            response,
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SearchResponseTypes {
    SuccessResponse(TsysPaymentsSyncResponse),
    ErrorResponse(TsysErrorResponse),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TsysSyncResponse {
    search_transaction_response: SearchResponseTypes,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, TsysSyncResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, TsysSyncResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let tsys_search_response = item.response.search_transaction_response;
        let (response, status) = match tsys_search_response {
            SearchResponseTypes::SuccessResponse(search_response) => (
                Ok(get_payments_sync_response(&search_response)),
                enums::AttemptStatus::from(search_response.transaction_details),
            ),
            SearchResponseTypes::ErrorResponse(connector_error_response) => (
                Err(get_error_response(connector_error_response, item.http_code)),
                item.data.status,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
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

impl From<TsysTransactionDetails> for enums::RefundStatus {
    fn from(item: TsysTransactionDetails) -> Self {
        match item.transaction_status {
            TsysTransactionStatus::Approved => Self::Pending,
            //Connector calls refunds as Void
            TsysTransactionStatus::Void => Self::Success,
            TsysTransactionStatus::Declined => Self::Failure,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefundResponse {
    return_response: TsysResponseTypes,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let tsys_return_response = item.response.return_response;
        let response = match tsys_return_response {
            TsysResponseTypes::SuccessResponse(return_response) => Ok(types::RefundsResponseData {
                connector_refund_id: return_response.transaction_id,
                refund_status: enums::RefundStatus::from(return_response.status),
            }),
            TsysResponseTypes::ErrorResponse(connector_error_response) => {
                Err(get_error_response(connector_error_response, item.http_code))
            }
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

impl TryFrom<types::RefundsResponseRouterData<api::RSync, TsysSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, TsysSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let tsys_search_response = item.response.search_transaction_response;
        let response = match tsys_search_response {
            SearchResponseTypes::SuccessResponse(search_response) => {
                Ok(types::RefundsResponseData {
                    connector_refund_id: search_response.transaction_details.transaction_id.clone(),
                    refund_status: enums::RefundStatus::from(search_response.transaction_details),
                })
            }
            SearchResponseTypes::ErrorResponse(connector_error_response) => {
                Err(get_error_response(connector_error_response, item.http_code))
            }
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}
