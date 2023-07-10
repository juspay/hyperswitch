use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::CardData,
    core::errors,
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TsysAuthPaymentsRequest {
    auth: TsysPaymentAuthSaleRequest,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TsysSalePaymentsRequest {
    sale: TsysPaymentAuthSaleRequest,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TsysPaymentAuthSaleRequest {
    #[serde(rename = "deviceID")]
    device_id: Secret<String>,
    transaction_key: Secret<String>,
    card_data_source: String,
    transaction_amount: String,
    card_number: cards::CardNumber,
    expiration_date: Secret<String>,
    cvv2: Secret<String>,
    terminal_capability: String,
    terminal_operating_environment: String,
    cardholder_authentication_method: String,
    #[serde(rename = "developerID")]
    developer_id: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for TsysAuthPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => {
                let connector_auth: TsysAuthType =
                    TsysAuthType::try_from(&item.connector_auth_type)?;
                let auth = TsysPaymentAuthSaleRequest {
                    device_id: connector_auth.device_id,
                    transaction_key: connector_auth.transaction_key,
                    card_data_source: "MANUAL".to_string(),
                    transaction_amount: item.request.amount.to_string(),
                    card_number: ccard.card_number.clone(),
                    expiration_date: ccard
                        .get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned()),
                    cvv2: ccard.card_cvc,
                    terminal_capability: "ICC_CHIP_READ_ONLY".to_string(),
                    terminal_operating_environment: "ON_MERCHANT_PREMISES_ATTENDED".to_string(),
                    cardholder_authentication_method: "NOT_AUTHENTICATED".to_string(),
                    developer_id: connector_auth.developer_id,
                };
                Ok(Self { auth })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for TsysSalePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => {
                let connector_auth: TsysAuthType =
                    TsysAuthType::try_from(&item.connector_auth_type)?;
                let sale = TsysPaymentAuthSaleRequest {
                    device_id: connector_auth.device_id,
                    transaction_key: connector_auth.transaction_key,
                    card_data_source: "MANUAL".to_string(),
                    transaction_amount: item.request.amount.to_string(),
                    card_number: ccard.card_number.clone(),
                    expiration_date: ccard
                        .get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned()),
                    cvv2: ccard.card_cvc,
                    terminal_capability: "ICC_CHIP_READ_ONLY".to_string(),
                    terminal_operating_environment: "ON_MERCHANT_PREMISES_ATTENDED".to_string(),
                    cardholder_authentication_method: "NOT_AUTHENTICATED".to_string(),
                    developer_id: connector_auth.developer_id,
                };
                Ok(Self { sale })
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]

pub struct TsysPaymentsAuthResponse {
    pub auth_response: AuthSaleResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TsysPaymentsSaleResponse {
    pub sale_response: AuthSaleResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthSaleResponse {
    pub status: TsysPaymentStatus,
    pub response_code: String,
    pub response_message: String,
    #[serde(rename = "transactionID")]
    pub transaction_id: String,
    pub transaction_amount: String,
    pub processed_amount: String,
    pub card_transaction_identifier: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, TsysPaymentsAuthResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            TsysPaymentsAuthResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = match item.response.auth_response.status {
            TsysPaymentStatus::Pass => enums::AttemptStatus::Authorized,
            TsysPaymentStatus::Fail => enums::AttemptStatus::AuthorizationFailed,
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.auth_response.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, TsysPaymentsSaleResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            TsysPaymentsSaleResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.sale_response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.sale_response.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysPSyncRequest {
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
pub struct TsysPaymentsSyncRequest {
    search_transaction: TsysPSyncRequest,
}

impl TryFrom<&types::PaymentsSyncRouterData> for TsysPaymentsSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let connector_auth: TsysAuthType = TsysAuthType::try_from(&item.connector_auth_type)?;
        let search_transaction = TsysPSyncRequest {
            device_id: connector_auth.device_id,
            transaction_key: connector_auth.transaction_key,
            transaction_id: item.request.connector_transaction_id.get_connector_transaction_id().change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TsysCancelStatus {
    Pass,
    Fail,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TsysCancelResponse {
    #[serde(rename = "transactionID")]
    transaction_id: String,
    status: TsysCancelStatus,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TsysPaymentsCancelResponse {
    void_response: TsysCancelResponse,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<F, TsysPaymentsCancelResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            TsysPaymentsCancelResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = match item.response.void_response.status {
            TsysCancelStatus::Pass => enums::AttemptStatus::Voided,
            TsysCancelStatus::Fail => enums::AttemptStatus::VoidFailed,
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.void_response.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysPaymentsCaptRequest {
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
    capture: TsysPaymentsCaptRequest,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for TsysPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let connector_auth: TsysAuthType = TsysAuthType::try_from(&item.connector_auth_type)?;
        let capture = TsysPaymentsCaptRequest {
            device_id: connector_auth.device_id,
            transaction_key: connector_auth.transaction_key,
            transaction_id: item.request.connector_transaction_id.clone(),
            developer_id: connector_auth.developer_id,
            transaction_amount: item.request.amount_to_capture.to_string(),
        };
        Ok(Self { capture })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysCaptureResponse {
    #[serde(rename = "transactionID")]
    transaction_id: String,
    status: TsysPaymentStatus,
    transaction_amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TsysPaymentCaptureResponse {
    capture_response: TsysCaptureResponse,
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<TsysPaymentCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<TsysPaymentCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let amount_captured = item.data.request.amount_to_capture;
        let status = enums::AttemptStatus::from(item.response.capture_response.status);

        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.capture_response.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            amount_captured: Some(amount_captured),
            ..item.data
        })
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

#[derive( Debug, Serialize)]
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
        Ok(Self {
            return_request,
        })
    }
}

// Type definition for Refund Response
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Pass,
    Fail
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Pass => Self::Success,
            RefundStatus::Fail => Self::Failure,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsysReturnResponse {
    #[serde(rename = "transactionID")]
    transaction_id: String,
    status: RefundStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefundResponse {
    return_response: TsysReturnResponse,
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
                connector_refund_id: item.response.return_response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.return_response.status),
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
                connector_refund_id: item.response.return_response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.return_response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// #[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
// pub struct TsysErrorResponse {
//     pub status_code: u16,
//     pub code: String,
//     pub message: String,
//     pub reason: Option<String>,
// }
// "status": "FAIL",
//         "responseCode": "F9901",
//         "responseMessage": "The value of element 'expirationDate' is not vali
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    status : TsysPaymentStatus,
    response_code: String,
    response_message: String

}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TsysAuthErrorResponse {
    auth_response: ErrorResponse
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TsysSaleErrorResponse {
    sale_response: ErrorResponse
}

#[derive( Debug,Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TsysErrorResponse {
    AuthErrorResponse(TsysAuthErrorResponse),
    SaleErrorResponse(TsysSaleErrorResponse),


}
