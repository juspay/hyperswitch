use common_utils::pii;
use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    connector::utils,
    types::{self, api, storage::enums, ConnectorAuthType},
};

#[derive(Debug, Deserialize , Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Auth,
    Capture,
    Credit,
    Refund,
    Sale,
    Update,
    Validate,
    Void
}

// Auth Struct
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


#[derive(Debug, Serialize)]
pub struct NmiPaymentsRequest {
    #[serde(rename = "type")]
    amount: f64,
    ccexp: Secret<String>,
    ccnumber: Secret<String, pii::CardNumber>,
    currency: enums::Currency,
    cvv: Secret<String>,
    security_key: String,
    transaction_type: TransactionType
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match &item.request.payment_method_data {
            api::PaymentMethodData::Card(card) => {
                let expiry_year = card.card_exp_year.peek().clone();
                let secret_value = format!(
                    "{}{}",
                    card.card_exp_month.peek(),
                    &expiry_year[expiry_year.len() - 2..]
                );
                let expiry_date: Secret<String> = Secret::new(secret_value);
                let transaction_type = match item.request.capture_method {
                    Some(storage_models::enums::CaptureMethod::Automatic) => TransactionType::Sale,
                    Some(storage_models::enums::CaptureMethod::Manual) => TransactionType::Auth,
                    _ => Err(errors::ConnectorError::NotImplemented(
                        "Capture Method".to_string(),
                    ))?,
                };
                let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
                Ok(Self {
                    transaction_type,
                    security_key: security_key.api_key,
                    amount: utils::convert_to_higher_denomination(
                        item.request.amount,
                        item.request.currency,
                    )?,
                    currency: item.request.currency,
                    ccnumber: card.card_number.clone(),
                    ccexp: expiry_date,
                    cvv: card.card_cvc.clone(),
                })
            }
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
        match &item.request.payment_method_data {
            api::PaymentMethodData::Card(card) => {
                let expiry_year = card.card_exp_year.peek().clone();
                let secret_value = format!(
                    "{}{}",
                    card.card_exp_month.peek(),
                    &expiry_year[expiry_year.len() - 2..]
                );
                let expiry_date: Secret<String> = Secret::new(secret_value);
                let transaction_type = TransactionType::Validate;
                let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
                Ok(Self {
                    transaction_type,
                    security_key: security_key.api_key,
                    amount: 0.0,
                    currency: item.request.currency,
                    ccnumber: card.card_number.clone(),
                    ccexp: expiry_date,
                    cvv: card.card_cvc.clone(),
                })
            }
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

impl TryFrom<(&types::PaymentsCaptureData, ConnectorAuthType)> for NmiCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: (&types::PaymentsCaptureData, ConnectorAuthType),
    ) -> Result<Self, Self::Error> {
        let security_key: NmiAuthType = (&item.1).try_into()?;
        let item = item.0;
        let security_key = security_key.api_key;

        Ok(Self {
            transaction_type: TransactionType::Capture,
            security_key,
            transactionid: item.connector_transaction_id.clone(),
            amount: Some(utils::convert_to_higher_denomination(
                item.amount,
                item.currency,
            )?),
        })
    }
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::Capture,
            GenericResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    >
    for types::RouterData<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Capture,
            GenericResponse,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status: enums::AttemptStatus = match item.response.response {
            Response::Approved => enums::AttemptStatus::CaptureInitiated,
            Response::Declined | Response::Error => enums::AttemptStatus::CaptureFailed,
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.transactionid),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
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

impl TryFrom<(&types::PaymentsCancelData, ConnectorAuthType)> for NmiCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: (&types::PaymentsCancelData, ConnectorAuthType),
    ) -> Result<Self, Self::Error> {
        let security_key: NmiAuthType = (&item.1).try_into()?;
        let item = item.0;
        let security_key = security_key.api_key;

        Ok(Self {
            transaction_type: TransactionType::Void,
            security_key,
            transactionid: item.connector_transaction_id.clone(),
            void_reason: item.cancellation_reason.clone(),
        })
    }
}

#[derive(Debug, Deserialize)]
pub enum Response {
    #[serde(alias = "1")]
    Approved,
    #[serde(alias = "2")]
    Declined,
    #[serde(alias = "3")]
    Error,
}

#[derive(Debug, Deserialize)]
pub struct GenericResponse {
    pub response: Response,
    pub responsetext: Option<String>,
    pub authcode: Option<String>,
    pub transactionid: String,
    pub avsresponse: Option<String>,
    pub cvvresponse: Option<String>,
    pub orderid: String,
    pub response_code: Option<String>,
}

impl<T>
    TryFrom<types::ResponseRouterData<api::Verify, GenericResponse, T, types::PaymentsResponseData>>
    for types::RouterData<api::Verify, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Verify,
            GenericResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status: enums::AttemptStatus = match item.response.response {
            Response::Approved => enums::AttemptStatus::Charged,
            Response::Declined | Response::Error => enums::AttemptStatus::Failure,
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.transactionid),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NmiPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<NmiPaymentStatus> for enums::AttemptStatus {
    fn from(item: NmiPaymentStatus) -> Self {
        match item {
            NmiPaymentStatus::Succeeded => Self::Charged,
            NmiPaymentStatus::Failed => Self::Failure,
            NmiPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NmiPaymentsResponse {
    status: NmiPaymentStatus,
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

impl
    TryFrom<
        types::ResponseRouterData<
            api::Authorize,
            GenericResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    >
    for types::RouterData<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::Authorize,
            GenericResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status: enums::AttemptStatus = match item.response.response {
            Response::Approved => match item.data.request.capture_method.unwrap_or_default() {
                storage_models::enums::CaptureMethod::Automatic => {
                    enums::AttemptStatus::Authorizing
                }
                _ => enums::AttemptStatus::CaptureInitiated,
            },
            Response::Declined | Response::Error => enums::AttemptStatus::Failure,
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.transactionid),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl<T>
    TryFrom<types::ResponseRouterData<api::Void, GenericResponse, T, types::PaymentsResponseData>>
    for types::RouterData<api::Void, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<api::Void, GenericResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status: enums::AttemptStatus = match item.response.response {
            Response::Approved => enums::AttemptStatus::VoidInitiated,
            Response::Declined | Response::Error => enums::AttemptStatus::VoidFailed,
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.transactionid),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Abandoned,
    Canceled,
    Pendingsettlement,
    Pending,
    Failed,
    Complete,
    InProgress,
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub condition: Condition,
    pub transaction_id: String,
}

#[derive(Debug, Deserialize)]
pub struct NMResponse {
    pub transaction: Transaction,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub nm_response: NMResponse,
}

impl TryFrom<types::PaymentsSyncResponseRouterData<QueryResponse>>
    for types::PaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsSyncResponseRouterData<QueryResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.nm_response.transaction.condition),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.nm_response.transaction.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

impl From<Condition> for enums::AttemptStatus {
    fn from(item: Condition) -> Self {
        match item {
            Condition::Abandoned => Self::AuthorizationFailed,
            Condition::Canceled => Self::Voided,
            Condition::Pendingsettlement | Condition::Pending => Self::Pending,
            Condition::Complete => Self::Charged,
            Condition::InProgress => Self::Pending,
            Condition::Failed | Condition::Unknown => Self::Failure,
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
    fn try_from(
        item: &types::RefundsRouterData<F>
    ) -> Result<Self, Self::Error> {
        let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let security_key = security_key.api_key;

        Ok(Self {
            transaction_type: TransactionType::Refund,
            security_key,
            transactionid: item.request.connector_transaction_id.clone(),
            amount: utils::convert_to_higher_denomination(
                item.request.refund_amount,
                item.request.currency)?,
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, GenericResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, GenericResponse>,
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

impl TryFrom<types::RefundsResponseRouterData<api::Capture, GenericResponse>>
    for types::RefundsRouterData<api::Capture>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Capture, GenericResponse>,
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
        let transaction_id = match item.request.connector_refund_id.clone() {
            Some(value) => Ok(value),
            None => Err(errors::ConnectorError::MissingConnectorRefundID),
        }?;

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
        let refund_status =
            enums::RefundStatus::from(item.response.nm_response.transaction.condition);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.nm_response.transaction.transaction_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<Condition> for enums::RefundStatus {
    fn from(item: Condition) -> Self {
        match item {
            Condition::Abandoned | Condition::Canceled | Condition::Failed | Condition::Unknown => {
                Self::Failure
            }
            Condition::Pendingsettlement | Condition::Pending | Condition::InProgress => {
                Self::Pending
            }
            Condition::Complete => Self::Success,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct NmiErrorResponse {
    pub error_code: String,
}

pub fn get_query_info(query_response: String) -> Result<(String, String), errors::ConnectorError> {
    let transaction_id_regex = Regex::new("<transaction_id>(.*)</transaction_id>")
        .map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?;
    let mut transaction_id = None;
    for tid in transaction_id_regex.captures_iter(&query_response) {
        transaction_id = Some(tid[1].to_string());
    }

    let condition_rejex = Regex::new("<condition>(.*)</condition>")
        .map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?;
    let mut condition = None;
    for cid in condition_rejex.captures_iter(&query_response) {
        condition = Some(cid[1].to_string());
    }

    let transaction_id = match transaction_id {
        Some(value) => Ok(value),
        None => Err(errors::ConnectorError::ResponseHandlingFailed),
    }?;

    let condition = match condition {
        Some(value) => Ok(value),
        None => Err(errors::ConnectorError::ResponseHandlingFailed),
    }?;
    Ok((transaction_id, condition))
}

pub fn get_attempt_status(
    value: String,
    capture_method: Option<enums::CaptureMethod>,
) -> Result<enums::AttemptStatus, errors::ConnectorError> {
    match value.as_str() {
        "abandoned" => Ok(enums::AttemptStatus::AuthorizationFailed),
        "canceled" => Ok(enums::AttemptStatus::Voided),
        "pending" => match capture_method.unwrap_or_default() {
            storage_models::enums::CaptureMethod::Manual => Ok(enums::AttemptStatus::Authorized),
            _ => Ok(enums::AttemptStatus::Pending),
        },
        "in_progress" => Ok(enums::AttemptStatus::Pending),
        "pendingsettlement" | "complete" => Ok(enums::AttemptStatus::Charged),
        "failed" | "unknown" => Ok(enums::AttemptStatus::Failure),
        _ => Err(errors::ConnectorError::ResponseHandlingFailed),
    }
}

pub fn get_refund_status(value: String) -> Result<enums::RefundStatus, errors::ConnectorError> {
    match value.as_str() {
        "abandoned" | "canceled" | "failed" | "unknown" => Ok(enums::RefundStatus::Failure),
        "pending" | "in_progress" => Ok(enums::RefundStatus::Pending),
        "pendingsettlement" | "complete" => Ok(enums::RefundStatus::Success),
        _ => Err(errors::ConnectorError::ResponseHandlingFailed),
    }
}
