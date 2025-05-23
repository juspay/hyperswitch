use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use common_enums::enums;
use common_utils::types::{AmountConvertor, MinorUnit, StringMajorUnit}; // Added StringMajorUnit
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{
        payments::Authorize as AuthorizeFlow, payments::Capture as CaptureFlow,
        payments::PSync as PSyncFlow, refunds::Execute, refunds::RSync,
    },
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCaptureData, PaymentsSyncData, RefundsData, ResponseId, // Added RefundsData
    }, 
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    // types::RefundsRouterData, // This is an alias for RouterData<F, RefundsData, RefundsResponseData>
};
use hyperswitch_interfaces::errors;

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

pub struct SpreedlyRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for SpreedlyRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize, Default)]
pub struct SpreedlyTransactionOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    capture: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SpreedlyAuthorizeRequestTransaction {
    #[serde(rename = "type")]
    transaction_type: String,
    amount: String,
    payment_method_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    currency_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<SpreedlyTransactionOptions>,
}

#[derive(Debug, Serialize)]
pub struct SpreedlyAuthorizeRequest {
    transaction: SpreedlyAuthorizeRequestTransaction,
}

// SpreedlyRouterData now takes &RouterData directly, not &PaymentsAuthorizeRouterData
// This makes it more generic if needed, but for Authorize, router_data is effectively PaymentsAuthorizeRouterData
impl TryFrom<&SpreedlyRouterData<&RouterData<AuthorizeFlow, PaymentsAuthorizeData, PaymentsResponseData>>>
    for SpreedlyAuthorizeRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &SpreedlyRouterData<&RouterData<AuthorizeFlow, PaymentsAuthorizeData, PaymentsResponseData>>,
    ) -> Result<Self, Self::Error> {
        let router_data = item.router_data;
        let payment_method_token = match router_data.payment_method_token.clone() {
            Some(hyperswitch_domain_models::router_data::PaymentMethodToken::Token(token)) => {
                token.expose()
            }
            _ => {
                return Err(report!(errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_method_token.token",
                }));
            }
        };

        let converter = common_utils::types::StringMajorUnitForConnector;
        let amount_in_major_unit_smu = converter
            .convert(item.amount, router_data.request.currency)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let amount_in_major_unit = amount_in_major_unit_smu.get_amount_as_string();

        let transaction = SpreedlyAuthorizeRequestTransaction {
            transaction_type: "authorize".to_string(),
            amount: amount_in_major_unit,
            payment_method_token,
            currency_code: Some(router_data.request.currency.to_string()),
            options: Some(SpreedlyTransactionOptions { capture: Some(false) }),
        };
        Ok(Self { transaction })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct SpreedlyCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
}

pub struct SpreedlyAuthType {
    pub environment_key: Secret<String>,
    pub access_secret: Secret<String>,
    pub gateway_token: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SpreedlyAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                environment_key: api_key.to_owned(),
                access_secret: key1.to_owned(),
                gateway_token: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SpreedlyTransactionState {
    Succeeded,
    #[default]
    Pending,
    PendingCapture,
    Failed,
    GatewayProcessingFailed,
    Retained,
    Redacted,
    #[serde(other)]
    Unknown,
}

impl From<SpreedlyTransactionState> for common_enums::AttemptStatus {
    fn from(item: SpreedlyTransactionState) -> Self {
        match item {
            SpreedlyTransactionState::Succeeded => common_enums::AttemptStatus::Authorized,
            SpreedlyTransactionState::PendingCapture => common_enums::AttemptStatus::Authorized,
            SpreedlyTransactionState::Pending => common_enums::AttemptStatus::Pending,
            SpreedlyTransactionState::Failed
            | SpreedlyTransactionState::GatewayProcessingFailed => common_enums::AttemptStatus::Failure,
            SpreedlyTransactionState::Retained
            | SpreedlyTransactionState::Redacted
            | SpreedlyTransactionState::Unknown => common_enums::AttemptStatus::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyAuthorizeResponsePaymentMethod {
    token: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyAuthorizeTransactionResponse {
    token: String,
    succeeded: bool,
    state: SpreedlyTransactionState,
    amount: StringMajorUnit, // Changed from String
    currency_code: String,
    #[serde(rename = "type")]
    transaction_type: String,
    payment_method: Option<SpreedlyAuthorizeResponsePaymentMethod>,
    gateway_transaction_id: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    message: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyAuthorizeResponse {
    transaction: SpreedlyAuthorizeTransactionResponse,
}

impl
    TryFrom<
        ResponseRouterData<
            AuthorizeFlow,
            SpreedlyAuthorizeResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<AuthorizeFlow, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            AuthorizeFlow,
            SpreedlyAuthorizeResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let converter = common_utils::types::StringMajorUnitForConnector;
        // item.response.transaction.amount is now StringMajorUnit
        let amount_minor_unit = converter
            .convert_back(item.response.transaction.amount.clone(), item.data.request.currency)
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.transaction.state.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction.token.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: item.response.transaction.gateway_transaction_id.clone(),
                connector_response_reference_id: Some(item.response.transaction.token.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            amount_captured: Some(amount_minor_unit.get_amount_as_i64()),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            PSyncFlow,
            SpreedlyAuthorizeResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    > for RouterData<PSyncFlow, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            PSyncFlow,
            SpreedlyAuthorizeResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let converter = common_utils::types::StringMajorUnitForConnector;
        // item.response.transaction.amount is now StringMajorUnit
        let amount_minor_unit = converter
            .convert_back(item.response.transaction.amount.clone(), item.data.request.currency)
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.transaction.state.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction.token.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: item.response.transaction.gateway_transaction_id.clone(),
                connector_response_reference_id: Some(item.response.transaction.token.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            amount_captured: Some(amount_minor_unit.get_amount_as_i64()),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            CaptureFlow,
            SpreedlyAuthorizeResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<CaptureFlow, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            CaptureFlow,
            SpreedlyAuthorizeResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let converter = common_utils::types::StringMajorUnitForConnector;
        // item.response.transaction.amount is now StringMajorUnit
        let amount_minor_unit = converter
            .convert_back(item.response.transaction.amount.clone(), item.data.request.currency)
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.transaction.state.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction.token.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: item.response.transaction.gateway_transaction_id.clone(),
                connector_response_reference_id: Some(item.response.transaction.token.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            amount_captured: Some(amount_minor_unit.get_amount_as_i64()),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct SpreedlyRefundRequest {
    pub amount: String,
}

// Changed RefundsRouterData to RouterData<F, RefundsData, RefundsResponseData> for consistency
impl<F> TryFrom<&SpreedlyRouterData<&RouterData<F, RefundsData, RefundsResponseData>>>
    for SpreedlyRefundRequest
where
    F: Clone, // May need more specific bounds if request.currency is accessed differently for F
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &SpreedlyRouterData<&RouterData<F, RefundsData, RefundsResponseData>>,
    ) -> Result<Self, Self::Error> {
        let converter = common_utils::types::StringMajorUnitForConnector;
        let amount_major_smu = converter
            .convert(item.amount, item.router_data.request.currency) // Assuming F makes .request.currency valid
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Self {
            amount: amount_major_smu.get_amount_as_string(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>>
    for RouterData<Execute, RefundsData, RefundsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>>
    for RouterData<RSync, RefundsData, RefundsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub message: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyTransactionErrorResponse {
    pub succeeded: bool,
    pub state: Option<SpreedlyTransactionState>,
    pub message: Option<String>,
    pub gateway_transaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<SpreedlyError>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyErrorResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<SpreedlyTransactionErrorResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub succeeded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<SpreedlyError>>,
    #[serde(skip_serializing)]
    pub status_code: u16,
    #[serde(skip_serializing)]
    pub code: Option<String>,
    #[serde(skip_serializing)]
    pub reason: Option<String>,
}
