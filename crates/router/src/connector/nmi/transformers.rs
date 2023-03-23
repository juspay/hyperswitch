use common_utils::{errors::CustomResult, pii};
use error_stack::{IntoReport, ResultExt};
use masking::{Secret};
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

#[derive(Debug, Deserialize, Serialize)]
pub struct NMICard {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    amount: f64,
    ccnumber: Secret<String, pii::CardNumber>,
    currency: enums::Currency,
    ccexp: Secret<String>,
    cvv: Secret<String>,
    security_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GPayWalletPayment {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub googlepay_payment_data: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct APayWalletPayment {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub applepay_payment_data: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PaymentType {
    Card,
    #[serde(rename = "googlepay")]
    Googlepay,
    #[serde(rename = "applepay")]
    Applepay,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NMIPaymentMethod {
    Card(NMICard),
    GooglePay(GPayWalletPayment),
    ApplePay(APayWalletPayment),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct NmiPaymentsRequest {
    pub payment_method: NMIPaymentMethod
}

fn select_authorize_payment_method(
    item:&types::PaymentsAuthorizeRouterData
) -> CustomResult<NMIPaymentMethod, errors::ConnectorError> {
    match &item.request.payment_method_data {
        api::PaymentMethodData::Card(card) => {
            let secret_value = utils::CardData::get_card_expiry_month_year_2_digit_with_delimiter(
                card, 
                "".to_string());
            let expiry_date: Secret<String> = Secret::new(secret_value);
            let transaction_type = match item.request.capture_method {
                Some(storage_models::enums::CaptureMethod::Automatic) => TransactionType::Sale,
                Some(storage_models::enums::CaptureMethod::Manual) => TransactionType::Auth,
                _ => Err(errors::ConnectorError::NotImplemented(
                    "Capture Method".to_string(),
                ))?,
            };
            let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
            Ok(NMIPaymentMethod::Card(NMICard {
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
            }))
        },
        api::PaymentMethodData::Wallet(wallet) => match wallet {
            api_models::payments::WalletData::GooglePay(data) => {
                Ok(NMIPaymentMethod::GooglePay(GPayWalletPayment {
                    payment_type: PaymentType::Googlepay,
                    googlepay_payment_data: data.tokenization_data.token.to_owned(),
                }))
            }
            api_models::payments::WalletData::ApplePay(data) => {
                Ok(NMIPaymentMethod::ApplePay(APayWalletPayment {
                    payment_type: PaymentType::Applepay,
                    applepay_payment_data: data.payment_data.to_owned(),
                }))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Wallet Type".to_string()).into()),
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment Method".to_string(),
        ))
        .into_report(),
    }
}

fn select_verify_payment_method(
    item:&types::VerifyRouterData
) -> CustomResult<NMIPaymentMethod, errors::ConnectorError> {
    match &item.request.payment_method_data {
        api::PaymentMethodData::Card(card) => { 
            let secret_value = utils::CardData::get_card_expiry_month_year_2_digit_with_delimiter(
                card, 
                "".to_string());
            let expiry_date: Secret<String> = Secret::new(secret_value);
            let transaction_type = TransactionType::Validate;
            let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
            Ok(NMIPaymentMethod::Card(NMICard {
                transaction_type,
                security_key: security_key.api_key,
                amount: 0.0,
                currency: item.request.currency,
                ccnumber: card.card_number.clone(),
                ccexp: expiry_date,
                cvv: card.card_cvc.clone(),
            }))
        },
        api::PaymentMethodData::Wallet(wallet) => match wallet {
            api_models::payments::WalletData::GooglePay(data) => {
                Ok(NMIPaymentMethod::GooglePay(GPayWalletPayment {
                    payment_type: PaymentType::Googlepay,
                    googlepay_payment_data: data.tokenization_data.token.to_owned(),
                }))
            }
            api_models::payments::WalletData::ApplePay(data) => {
                Ok(NMIPaymentMethod::ApplePay(APayWalletPayment {
                    payment_type: PaymentType::Applepay,
                    applepay_payment_data: data.payment_data.to_owned(),
                }))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Wallet Type".to_string()).into()),
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment Method".to_string(),
        ))
        .into_report(),
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method: select_authorize_payment_method(item)?
        })
    }
}

impl TryFrom<&types::VerifyRouterData> for NmiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::VerifyRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method: select_verify_payment_method(item)?
        })
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
    fn try_from(
        item: &types::PaymentsCaptureRouterData,
    ) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            transaction_type: TransactionType::Capture,
            security_key: auth.api_key,
            transactionid: item.request.connector_transaction_id.clone(),
            amount: Some(utils::convert_to_higher_denomination(
                item.request.amount,
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

impl TryFrom<&types::PaymentsCancelRouterData> for NmiCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &types::PaymentsCancelRouterData,
    ) -> Result<Self, Self::Error> {
        let auth = NmiAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            transaction_type: TransactionType::Void,
            security_key: auth.api_key,
            transactionid: item.request.connector_transaction_id.clone(),
            void_reason: item.request.cancellation_reason.clone(),
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
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
    pub response_code: u16,
}

impl<T>
    TryFrom<types::ResponseRouterData<api::Verify, StandardResponse, T, types::PaymentsResponseData>>
    for types::RouterData<api::Verify, T, types::PaymentsResponseData>
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
            StandardResponse,
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
            StandardResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status: enums::AttemptStatus = match item.response.response {
            Response::Approved => match item.data.request.capture_method.unwrap_or_default() {
                storage_models::enums::CaptureMethod::Automatic => {
                    enums::AttemptStatus::CaptureInitiated
                }
                _ => enums::AttemptStatus::Authorized,
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
    TryFrom<types::ResponseRouterData<api::Void, StandardResponse, T, types::PaymentsResponseData>>
    for types::RouterData<api::Void, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<api::Void, StandardResponse, T, types::PaymentsResponseData>,
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

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
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
    pub condition: Condition,
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
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.nm_response.transaction.transaction_id),
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
            Condition::Cancelled => Self::Voided,
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
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let security_key: NmiAuthType = (&item.connector_auth_type).try_into()?;
        let security_key = security_key.api_key;

        Ok(Self {
            transaction_type: TransactionType::Refund,
            security_key,
            transactionid: item.request.connector_transaction_id.clone(),
            amount: utils::convert_to_higher_denomination(
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
            Condition::Abandoned | Condition::Cancelled | Condition::Failed | Condition::Unknown => {
                Self::Failure
            }
            Condition::Pendingsettlement | Condition::Pending | Condition::InProgress => {
                Self::Pending
            }
            Condition::Complete => Self::Success,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NmiErrorResponse {
    pub error_code: String,
}

// This function is a temporary fix for future that will looked upon.
pub fn get_query_info(query_response: String) -> Result<(String, Condition), errors::ConnectorError> {
    let transaction_id_regex = Regex::new("<transaction_id>(.*)</transaction_id>")
        .map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?;
    let mut transaction_id = None;
    for tid in transaction_id_regex.captures_iter(&query_response) {
        transaction_id = Some(tid[1].to_string());
    }

    let condition_rejex = Regex::new("<condition>(.*)</condition>")
        .map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?;
    let mut condition: Option<Condition> = Some(Condition::InProgress);
    for cid in condition_rejex.captures_iter(&query_response) {
        condition = match &cid[1] {
            "abandoned" => Some(Condition::Abandoned),
            "cancelled" => Some(Condition::Cancelled),
            "pending" => Some(Condition::Pending),
            "in_progress" => Some(Condition::InProgress),
            "pendingsettlement" => Some(Condition::Pendingsettlement),
            "complete" => Some(Condition::Complete),
            "failed" => Some(Condition::Failed),
            "unknown" => Some(Condition::Unknown),
            _ => None,
        };
    }

    let transaction_id = match transaction_id {
        Some(value) => Ok(value),
        None => Err(errors::ConnectorError::ResponseHandlingFailed),
    }?;

    let condition = match condition {
        Some(value) => Ok(value),
        None => Err(errors::ConnectorError::ResponseHandlingFailed)
    }?;
    Ok((transaction_id, condition))
}

// pub fn get_attempt_status(
//     value: Condition,
//     capturemethod: Option<enums::CaptureMethod>,
// ) -> Result<enums::AttemptStatus, errors::ConnectorError> {
//     match value {
//         Condition::Abandoned => Ok(enums::AttemptStatus::AuthorizationFailed),
//         Condition::Cancelled => Ok(enums::AttemptStatus::Voided),
//         Condition::Pending => match capturemethod.unwrap_or_default() {
//             storage_models::enums::CaptureMethod::Manual => Ok(enums::AttemptStatus::Authorized),
//             _ => Ok(enums::AttemptStatus::Pending),
//         },
//         Condition::InProgress => Ok(enums::AttemptStatus::Pending),
//         Condition::Pendingsettlement | Condition::Complete => Ok(enums::AttemptStatus::Charged),
//         Condition::Failed | Condition::Unknown => Ok(enums::AttemptStatus::Failure)
//     }
// }

pub fn get_refund_status(value: Condition) -> Result<enums::RefundStatus, errors::ConnectorError> {
    match value {
        Condition::Abandoned | Condition::Cancelled | Condition::Failed | Condition::Unknown => Ok(enums::RefundStatus::Failure),
        Condition::Pending | Condition::InProgress => Ok(enums::RefundStatus::Pending),
        Condition::Pendingsettlement | Condition::Complete => Ok(enums::RefundStatus::Success)
    }
}
