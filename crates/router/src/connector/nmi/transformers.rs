use common_utils::pii;
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils,
    core::errors,
    types::{self, api, storage::enums, ConnectorAuthType},
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Auth,
    Capture,
    Credit,
    Refund,
    Sale,
    Update,
    Validate,
    Void,
}

pub struct NmiAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&ConnectorAuthType> for NmiAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreeDSCondition {
    Verified,
    Attempted,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NmiPaymentsRequest {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    amount: f64,
    security_key: String,
    // Card
    ccnumber: Option<Secret<String, pii::CardNumber>>,
    currency: enums::Currency,
    ccexp: Option<Secret<String>>,
    cvv: Option<Secret<String>>,
    // Payment method types
    applepay_payment_data: Option<String>,
    googlepay_payment_data: Option<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let transaction_type = match item.request.capture_method {
            Some(storage_models::enums::CaptureMethod::Automatic) => TransactionType::Sale,
            Some(storage_models::enums::CaptureMethod::Manual) => TransactionType::Auth,
            _ => Err(errors::ConnectorError::NotImplemented(
                "Capture Method".to_string(),
            ))?,
        };
        let auth_type: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let amount =
            utils::to_currency_base_unit_as_f64(item.request.amount, item.request.currency)?;
        match &item.request.payment_method_data {
            api::PaymentMethodData::Card(ref card) => {
                let value = utils::CardData::get_card_expiry_month_year_2_digit_with_delimiter(
                    card,
                    "".to_string(),
                );
                let expiry_date: Secret<String> = value;
                Ok(Self {
                    transaction_type,
                    security_key: auth_type.api_key,
                    amount,
                    currency: item.request.currency,
                    ccnumber: Some(card.card_number.clone()),
                    ccexp: Some(expiry_date),
                    cvv: Some(card.card_cvc.clone()),
                    applepay_payment_data: None,
                    googlepay_payment_data: None,
                })
            }
            api::PaymentMethodData::Wallet(ref wallet_type) => match wallet_type {
                api_models::payments::WalletData::GooglePay(ref googlepay_data) => Ok(Self {
                    transaction_type,
                    security_key: auth_type.api_key,
                    amount,
                    currency: item.request.currency,
                    ccnumber: None,
                    ccexp: None,
                    cvv: None,
                    applepay_payment_data: None,
                    googlepay_payment_data: Some(googlepay_data.tokenization_data.token.to_owned()),
                }),
                api_models::payments::WalletData::ApplePay(ref applepay_data) => Ok(Self {
                    transaction_type,
                    security_key: auth_type.api_key,
                    amount,
                    currency: item.request.currency,
                    ccnumber: None,
                    ccexp: None,
                    cvv: None,
                    applepay_payment_data: Some(applepay_data.payment_data.to_owned()),
                    googlepay_payment_data: None,
                }),
                _ => Err(errors::ConnectorError::NotImplemented("Wallet type".to_string()).into()),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))
            .into_report(),
        }
    }
}

impl TryFrom<&types::VerifyRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::VerifyRouterData) -> Result<Self, Self::Error> {
        let transaction_type = TransactionType::Validate;
        let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
        match &item.request.payment_method_data {
            api::PaymentMethodData::Card(ref card) => {
                let value = utils::CardData::get_card_expiry_month_year_2_digit_with_delimiter(
                    card,
                    "".to_string(),
                );
                let expiry_date: Secret<String> = value;
                Ok(Self {
                    transaction_type,
                    security_key: security_key.api_key,
                    amount: 0.0,
                    currency: item.request.currency,
                    ccnumber: Some(card.card_number.clone()),
                    ccexp: Some(expiry_date),
                    cvv: Some(card.card_cvc.clone()),
                    applepay_payment_data: None,
                    googlepay_payment_data: None,
                })
            }
            api::PaymentMethodData::Wallet(ref wallet_type) => match wallet_type {
                api_models::payments::WalletData::GooglePay(ref googlepay_data) => Ok(Self {
                    transaction_type,
                    security_key: security_key.api_key,
                    amount: 0.0,
                    currency: item.request.currency,
                    ccnumber: None,
                    ccexp: None,
                    cvv: None,
                    applepay_payment_data: None,
                    googlepay_payment_data: Some(googlepay_data.tokenization_data.token.to_owned()),
                }),
                api_models::payments::WalletData::ApplePay(ref applepay_data) => Ok(Self {
                    transaction_type,
                    security_key: security_key.api_key,
                    amount: 0.0,
                    currency: item.request.currency,
                    ccnumber: None,
                    ccexp: None,
                    cvv: None,
                    applepay_payment_data: Some(applepay_data.payment_data.to_owned()),
                    googlepay_payment_data: None,
                }),
                _ => Err(errors::ConnectorError::NotImplemented("Wallet type".to_string()).into()),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))
            .into_report(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NmiSyncRequest {
    pub transaction_id: String,
    pub security_key: String,
}

impl TryFrom<&types::PaymentsSyncRouterData> for NmiSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            security_key: auth.api_key,
            transaction_id: item
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmiCaptureRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub transactionid: String,
    pub amount: Option<f64>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for NmiCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            transaction_type: TransactionType::Capture,
            security_key: auth.api_key,
            transactionid: item.request.connector_transaction_id.clone(),
            amount: Some(utils::to_currency_base_unit_as_f64(
                item.request.amount_to_capture,
                item.request.currency,
            )?),
        })
    }
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::Capture,
            StandardResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Capture,
            StandardResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                }),
                enums::AttemptStatus::CaptureInitiated,
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse {
                    code: item.response.response_code,
                    message: item.response.responsetext,
                    reason: None,
                    status_code: item.http_code,
                }),
                enums::AttemptStatus::CaptureFailed,
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
pub struct NmiCancelRequest {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub security_key: String,
    pub transactionid: String,
    pub void_reason: Option<String>,
}

impl TryFrom<&types::PaymentsCancelRouterData> for NmiCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            transaction_type: TransactionType::Void,
            security_key: auth.api_key,
            transactionid: item.request.connector_transaction_id.clone(),
            void_reason: item.request.cancellation_reason.clone(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub enum Response {
    #[serde(alias = "1")]
    Approved,
    #[serde(alias = "2")]
    Declined,
    #[serde(alias = "3")]
    Error,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StandardResponse {
    pub response: Response,
    pub responsetext: String,
    pub authcode: Option<String>,
    pub transactionid: String,
    pub avsresponse: Option<String>,
    pub cvvresponse: Option<String>,
    pub orderid: String,
    pub response_code: String,
}

impl<T>
    TryFrom<
        types::ResponseRouterData<api::Verify, StandardResponse, T, types::PaymentsResponseData>,
    > for types::RouterData<api::Verify, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Verify,
            StandardResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                }),
                enums::AttemptStatus::Charged,
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse {
                    code: item.response.response_code,
                    message: item.response.responsetext,
                    reason: None,
                    status_code: item.http_code,
                }),
                enums::AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NmiResponseStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<NmiResponseStatus> for enums::AttemptStatus {
    fn from(item: NmiResponseStatus) -> Self {
        match item {
            NmiResponseStatus::Succeeded => Self::Charged,
            NmiResponseStatus::Failed => Self::Failure,
            NmiResponseStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NmiPaymentsResponse {
    status: NmiResponseStatus,
    id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, NmiPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, NmiPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::PaymentsResponseRouterData<StandardResponse>>
    for types::RouterData<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Authorize,
            StandardResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                }),
                if let Some(storage_models::enums::CaptureMethod::Automatic) =
                    item.data.request.capture_method
                {
                    enums::AttemptStatus::CaptureInitiated
                } else {
                    enums::AttemptStatus::Authorizing
                },
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse {
                    code: item.response.response_code,
                    message: item.response.responsetext,
                    reason: None,
                    status_code: item.http_code,
                }),
                enums::AttemptStatus::Failure,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<T>
    TryFrom<types::ResponseRouterData<api::Void, StandardResponse, T, types::PaymentsResponseData>>
    for types::RouterData<api::Void, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Void,
            StandardResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (response, status) = match item.response.response {
            Response::Approved => (
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        item.response.transactionid,
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                }),
                enums::AttemptStatus::VoidInitiated,
            ),
            Response::Declined | Response::Error => (
                Err(types::ErrorResponse {
                    code: item.response.response_code,
                    message: item.response.responsetext,
                    reason: None,
                    status_code: item.http_code,
                }),
                enums::AttemptStatus::VoidFailed,
            ),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NmiStatus {
    Abandoned,
    Cancelled,
    Pendingsettlement,
    Pending,
    Failed,
    Complete,
    InProgress,
    Unknown,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Transaction {
    #[serde(rename = "condition")]
    pub status: NmiStatus,
    pub transaction_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NMResponse {
    pub transaction: Transaction,
}

#[derive(Clone, Debug, Deserialize)]
pub struct QueryResponse {
    pub nm_response: NMResponse,
}

impl TryFrom<types::PaymentsSyncResponseRouterData<types::Response>>
    for types::PaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsSyncResponseRouterData<types::Response>,
    ) -> Result<Self, Self::Error> {
        let query_response = String::from_utf8(item.response.response.to_vec())
            .into_report()
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let response = get_query_info(query_response)?;
        Ok(Self {
            status: get_attempt_status(response.1, item.data.request.capture_method)?,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(response.0),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl From<NmiStatus> for enums::AttemptStatus {
    fn from(item: NmiStatus) -> Self {
        match item {
            NmiStatus::Abandoned => Self::AuthorizationFailed,
            NmiStatus::Cancelled => Self::Voided,
            NmiStatus::Pendingsettlement | NmiStatus::Pending => Self::Pending,
            NmiStatus::Complete => Self::Charged,
            NmiStatus::InProgress => Self::Pending,
            NmiStatus::Failed | NmiStatus::Unknown => Self::Failure,
        }
    }
}

// REFUND :
#[derive(Debug, Serialize)]
pub struct NmiRefundRequest {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    security_key: String,
    transactionid: String,
    amount: f64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for NmiRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let security_key = security_key.api_key;

        Ok(Self {
            transaction_type: TransactionType::Refund,
            security_key,
            transactionid: item.request.connector_transaction_id.clone(),
            amount: utils::to_currency_base_unit_as_f64(
                item.request.refund_amount,
                item.request.currency,
            )?,
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, StandardResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, StandardResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transactionid,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Capture, StandardResponse>>
    for types::RefundsRouterData<api::Capture>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Capture, StandardResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transactionid,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<Response> for enums::RefundStatus {
    fn from(item: Response) -> Self {
        match item {
            Response::Approved => Self::Pending,
            Response::Declined | Response::Error => Self::Failure,
        }
    }
}

impl TryFrom<&types::RefundSyncRouterData> for NmiSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        let transaction_id = item
            .request
            .connector_refund_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorRefundID)?;

        Ok(Self {
            security_key: auth.api_key,
            transaction_id,
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, QueryResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, QueryResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.nm_response.transaction.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.nm_response.transaction.transaction_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<NmiStatus> for enums::RefundStatus {
    fn from(item: NmiStatus) -> Self {
        match item {
            NmiStatus::Abandoned
            | NmiStatus::Cancelled
            | NmiStatus::Failed
            | NmiStatus::Unknown => Self::Failure,
            NmiStatus::Pendingsettlement | NmiStatus::Pending | NmiStatus::InProgress => {
                Self::Pending
            }
            NmiStatus::Complete => Self::Success,
        }
    }
}

// This function is a temporary fix for future that will looked upon.
pub fn get_query_info(
    query_response: String,
) -> Result<(String, NmiStatus), errors::ConnectorError> {
    let transaction_id_regex = Regex::new("<transaction_id>(.*)</transaction_id>")
        .map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?;
    let mut transaction_id = None;
    for tid in transaction_id_regex.captures_iter(&query_response) {
        transaction_id = Some(tid[1].to_string());
    }
    let condition_regex = Regex::new("<condition>(.*)</condition>")
        .map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?;
    let mut nmi_status: Option<NmiStatus> = Some(NmiStatus::InProgress);
    for cid in condition_regex.captures_iter(&query_response) {
        nmi_status = match &cid[1] {
            "abandoned" => Some(NmiStatus::Abandoned),
            "canceled" => Some(NmiStatus::Cancelled),
            "pending" => Some(NmiStatus::Pending),
            "in_progress" => Some(NmiStatus::InProgress),
            "pendingsettlement" => Some(NmiStatus::Pendingsettlement),
            "complete" => Some(NmiStatus::Complete),
            "failed" => Some(NmiStatus::Failed),
            "unknown" => Some(NmiStatus::Unknown),
            _ => None,
        };
    }
    let transaction_id = match transaction_id {
        Some(value) => Ok(value),
        None => Err(errors::ConnectorError::ResponseHandlingFailed),
    }?;
    let nmi_status = match nmi_status {
        Some(value) => Ok(value),
        None => Err(errors::ConnectorError::ResponseHandlingFailed),
    }?;
    Ok((transaction_id, nmi_status))
}

pub fn get_attempt_status(
    value: NmiStatus,
    capturemethod: Option<enums::CaptureMethod>,
) -> Result<enums::AttemptStatus, errors::ConnectorError> {
    match value {
        NmiStatus::Abandoned => Ok(enums::AttemptStatus::AuthorizationFailed),
        NmiStatus::Cancelled => Ok(enums::AttemptStatus::Voided),
        NmiStatus::Pending => match capturemethod.unwrap_or_default() {
            storage_models::enums::CaptureMethod::Manual => Ok(enums::AttemptStatus::Authorized),
            _ => Ok(enums::AttemptStatus::Pending),
        },
        NmiStatus::InProgress => Ok(enums::AttemptStatus::Pending),
        NmiStatus::Pendingsettlement | NmiStatus::Complete => Ok(enums::AttemptStatus::Charged),
        NmiStatus::Failed | NmiStatus::Unknown => Ok(enums::AttemptStatus::Failure),
    }
}

pub fn get_refund_status(value: NmiStatus) -> Result<enums::RefundStatus, errors::ConnectorError> {
    match value {
        NmiStatus::Abandoned | NmiStatus::Cancelled | NmiStatus::Failed | NmiStatus::Unknown => {
            Ok(enums::RefundStatus::Failure)
        }
        NmiStatus::Pending | NmiStatus::InProgress => Ok(enums::RefundStatus::Pending),
        NmiStatus::Pendingsettlement | NmiStatus::Complete => Ok(enums::RefundStatus::Success),
    }
}
