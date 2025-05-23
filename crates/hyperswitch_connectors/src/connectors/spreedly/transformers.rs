use error_stack::{report, ResultExt};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use common_enums::enums;
use common_utils::types::{AmountConvertor, MinorUnit, StringMajorUnit}; // Added StringMajorUnit
use hyperswitch_domain_models::{
    payment_method_data, // Added for PaymentMethodData
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{
        payments::Authorize as AuthorizeFlow, payments::Capture as CaptureFlow,
        payments::PSync as PSyncFlow, payments::PaymentMethodToken, refunds::Execute,
        refunds::RSync,
    },
    router_request_types::{
        PaymentMethodTokenizationData, PaymentsAuthorizeData, PaymentsCaptureData,
        PaymentsSyncData, RefundsData, ResponseId, // Added RefundsData
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

        // For Capture, the status should reflect a successful capture.
        // Spreedly's 'succeeded' state for a capture transaction means it's captured.
        let capture_status = match item.response.transaction.state {
            SpreedlyTransactionState::Succeeded => common_enums::AttemptStatus::Charged,
            SpreedlyTransactionState::Failed | SpreedlyTransactionState::GatewayProcessingFailed => common_enums::AttemptStatus::Failure,
            _ => common_enums::AttemptStatus::Pending, // Or specific status like CaptureFailed if applicable
        };

        Ok(Self {
            status: capture_status,
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

// Request for Capture Flow
#[derive(Debug, Serialize, Default)]
pub struct SpreedlyCaptureRequestTransaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    amount: Option<String>, // Major unit string
    #[serde(skip_serializing_if = "Option::is_none")]
    currency_code: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct SpreedlyCaptureRequest {
    transaction: SpreedlyCaptureRequestTransaction,
}

impl TryFrom<&SpreedlyRouterData<&RouterData<CaptureFlow, PaymentsCaptureData, PaymentsResponseData>>>
    for SpreedlyCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &SpreedlyRouterData<&RouterData<CaptureFlow, PaymentsCaptureData, PaymentsResponseData>>,
    ) -> Result<Self, Self::Error> {
        let router_data = item.router_data;
        let amount_to_capture = router_data.request.amount_to_capture; // This is i64
        let currency = router_data.request.currency; // This is enums::Currency

        let (amount_str, currency_code_str) = if amount_to_capture > 0 {
            let converter = common_utils::types::StringMajorUnitForConnector;
            let major_unit_amount = converter
                .convert(MinorUnit::new(amount_to_capture), currency)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
            (
                Some(major_unit_amount.get_amount_as_string()),
                Some(currency.to_string()),
            )
        } else {
            // If amount_to_capture is 0 or not specified, Spreedly captures the full authorized amount
            // by not sending amount/currency in the request.
            (None, None)
        };


        Ok(Self {
            transaction: SpreedlyCaptureRequestTransaction {
                amount: amount_str,
                currency_code: currency_code_str,
            },
        })
    }
}


#[derive(Default, Debug, Serialize)]
pub struct SpreedlyRefundRequest {
    pub amount: String,
}

// Request for Tokenize Flow
#[derive(Debug, Serialize)]
pub struct SpreedlyTokenizeCard {
    first_name: Secret<String>,
    last_name: Secret<String>,
    number: cards::CardNumber,
    month: Secret<String>,
    year: Secret<String>,
    verification_value: Secret<String>, // CVV
}

#[derive(Debug, Serialize)]
pub struct SpreedlyTokenizeRequestPaymentMethod {
    credit_card: SpreedlyTokenizeCard,
    // email: Option<pii::Email>, // Spreedly docs show email at payment_method level
    // metadata: Option<serde_json::Value>
}

#[derive(Debug, Serialize)]
pub struct SpreedlyTokenizeRequest {
    payment_method: SpreedlyTokenizeRequestPaymentMethod,
    environment_key: Secret<String>,
}

impl TryFrom<&RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>>
    for SpreedlyTokenizeRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let card_details = match item.request.payment_method_data.clone() {
            payment_method_data::PaymentMethodData::Card(card) => Ok(card),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Only card tokenization is supported".to_string(),
            )),
        }?;

        let auth = SpreedlyAuthType::try_from(&item.connector_auth_type)?;

        Ok(Self {
            environment_key: auth.environment_key,
            payment_method: SpreedlyTokenizeRequestPaymentMethod {
                credit_card: SpreedlyTokenizeCard {
                    first_name: card_details.card_holder_name.clone().unwrap_or_else(|| Secret::new("".to_string())), // Spreedly requires first/last name
                    last_name: card_details.card_holder_name.unwrap_or_else(|| Secret::new("".to_string())), // Assuming full name is in card_holder_name
                    number: card_details.card_number,
                    month: card_details.card_exp_month,
                    year: card_details.card_exp_year,
                    verification_value: card_details.card_cvc,
                },
            },
        })
    }
}


// Response for Tokenize Flow
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyTokenizeResponsePaymentMethod {
    token: String,
    // ... other fields like created_at, updated_at, email, storage_state, test, etc.
    // For simplicity, only token is mapped for now.
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyTokenizeResponse {
    transaction: SpreedlyTokenizeResponsePaymentMethod, // Spreedly wraps it in a "transaction" object, even for tokenization
}


impl TryFrom<ResponseRouterData<PaymentMethodToken, SpreedlyTokenizeResponse, PaymentMethodTokenizationData, PaymentsResponseData>>
    for RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: ResponseRouterData<PaymentMethodToken, SpreedlyTokenizeResponse, PaymentMethodTokenizationData, PaymentsResponseData>) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.transaction.token,
            }),
            ..item.data
        })
    }
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
