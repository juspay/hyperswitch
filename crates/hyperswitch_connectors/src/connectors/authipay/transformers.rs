use common_enums::enums;
use common_utils::types::FloatMajorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, CardData},
};

// Type definition for router data with amount
pub struct AuthipayRouterData<T> {
    pub amount: FloatMajorUnit, // Amount in major units (e.g., dollars instead of cents)
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for AuthipayRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// Basic request/response structs used across multiple operations

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    total: FloatMajorUnit,
    currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    components: Option<AmountComponents>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmountComponents {
    subtotal: FloatMajorUnit,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExpiryDate {
    month: Secret<String>,
    year: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: cards::CardNumber,
    security_code: Secret<String>,
    expiry_date: ExpiryDate,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethod {
    payment_card: Card,
}

#[derive(Default, Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SplitShipment {
    total_count: i32,
    final_shipment: bool,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayPaymentsRequest {
    request_type: &'static str,
    transaction_amount: Amount,
    payment_method: PaymentMethod,
    // split_shipment: Option<SplitShipment>,
    // incremental_flag: Option<bool>,
}

impl TryFrom<&AuthipayRouterData<&PaymentsAuthorizeRouterData>> for AuthipayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthipayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        // Check if 3DS is being requested - Authipay doesn't support 3DS
        if matches!(
            item.router_data.auth_type,
            enums::AuthenticationType::ThreeDs
        ) {
            return Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("authipay"),
            )
            .into());
        }

        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let expiry_date = ExpiryDate {
                    month: req_card.card_exp_month.clone(),
                    year: req_card.get_card_expiry_year_2_digit()?,
                };

                let card = Card {
                    number: req_card.card_number.clone(),
                    security_code: req_card.card_cvc.clone(),
                    expiry_date,
                };

                let payment_method = PaymentMethod { payment_card: card };

                let transaction_amount = Amount {
                    total: item.amount,
                    currency: item.router_data.request.currency.to_string(),
                    components: None,
                };

                // Determine request type based on capture method
                let request_type = match item.router_data.request.capture_method {
                    Some(enums::CaptureMethod::Manual) => "PaymentCardPreAuthTransaction",
                    Some(enums::CaptureMethod::Automatic) => "PaymentCardSaleTransaction",
                    Some(enums::CaptureMethod::SequentialAutomatic) => "PaymentCardSaleTransaction",
                    Some(enums::CaptureMethod::ManualMultiple)
                    | Some(enums::CaptureMethod::Scheduled) => {
                        return Err(errors::ConnectorError::NotSupported {
                            message: "Capture method not supported by Authipay".to_string(),
                            connector: "Authipay",
                        }
                        .into());
                    }
                    None => "PaymentCardSaleTransaction", // Default when not specified
                };

                let request = Self {
                    request_type,
                    transaction_amount,
                    payment_method,
                };

                Ok(request)
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("authipay"),
            )
            .into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct AuthipayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for AuthipayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                api_secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// Transaction Status enum (like Fiserv's FiservPaymentStatus)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuthipayTransactionStatus {
    Authorized,
    Captured,
    Voided,
    Declined,
    Failed,
    #[default]
    Processing,
}

impl From<AuthipayTransactionStatus> for enums::AttemptStatus {
    fn from(item: AuthipayTransactionStatus) -> Self {
        match item {
            AuthipayTransactionStatus::Captured => Self::Charged,
            AuthipayTransactionStatus::Declined | AuthipayTransactionStatus::Failed => {
                Self::Failure
            }
            AuthipayTransactionStatus::Processing => Self::Pending,
            AuthipayTransactionStatus::Authorized => Self::Authorized,
            AuthipayTransactionStatus::Voided => Self::Voided,
        }
    }
}

// Transaction Processing Details (like Fiserv's TransactionProcessingDetails)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayTransactionProcessingDetails {
    pub order_id: String,
    pub transaction_id: String,
}

// Gateway Response (like Fiserv's GatewayResponse)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayGatewayResponse {
    pub transaction_state: AuthipayTransactionStatus,
    pub transaction_processing_details: AuthipayTransactionProcessingDetails,
}

// Payment Receipt (like Fiserv's PaymentReceipt)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayPaymentReceipt {
    pub approved_amount: Amount,
    pub processor_response_details: Option<Processor>,
}

// Main Response (like Fiserv's FiservPaymentsResponse) - but flat for JSON deserialization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayPaymentsResponse {
    #[serde(rename = "type")]
    response_type: Option<String>,
    client_request_id: String,
    api_trace_id: String,
    ipg_transaction_id: String,
    order_id: String,
    transaction_type: String,
    payment_token: Option<PaymentToken>,
    transaction_origin: Option<String>,
    payment_method_details: Option<PaymentMethodDetails>,
    country: Option<String>,
    terminal_id: Option<String>,
    merchant_id: Option<String>,
    transaction_time: i64,
    approved_amount: Amount,
    transaction_amount: Amount,
    // For payment transactions (SALE)
    transaction_status: Option<String>,
    // For refund transactions (RETURN)
    transaction_result: Option<String>,
    transaction_state: Option<String>,
    approval_code: String,
    scheme_transaction_id: Option<String>,
    processor: Processor,
}

impl AuthipayPaymentsResponse {
    /// Get gateway response (like Fiserv's gateway_response)
    pub fn gateway_response(&self) -> AuthipayGatewayResponse {
        AuthipayGatewayResponse {
            transaction_state: self.get_transaction_status(),
            transaction_processing_details: AuthipayTransactionProcessingDetails {
                order_id: self.order_id.clone(),
                transaction_id: self.ipg_transaction_id.clone(),
            },
        }
    }

    /// Get payment receipt (like Fiserv's payment_receipt)
    pub fn payment_receipt(&self) -> AuthipayPaymentReceipt {
        AuthipayPaymentReceipt {
            approved_amount: self.approved_amount.clone(),
            processor_response_details: Some(self.processor.clone()),
        }
    }

    /// Determine the transaction status based on transaction type and various status fields (like Fiserv)
    fn get_transaction_status(&self) -> AuthipayTransactionStatus {
        match self.transaction_type.as_str() {
            "RETURN" => {
                // Refund transaction - use transaction_result
                match self.transaction_result.as_deref() {
                    Some("APPROVED") => AuthipayTransactionStatus::Captured,
                    Some("DECLINED") | Some("FAILED") => AuthipayTransactionStatus::Failed,
                    _ => AuthipayTransactionStatus::Processing,
                }
            }
            "VOID" => {
                // Void transaction - use transaction_result, fallback to transaction_state
                match self.transaction_result.as_deref() {
                    Some("APPROVED") => AuthipayTransactionStatus::Voided,
                    Some("DECLINED") | Some("FAILED") => AuthipayTransactionStatus::Failed,
                    Some("PENDING") | Some("PROCESSING") => AuthipayTransactionStatus::Processing,
                    _ => {
                        // Fallback to transaction_state for void operations
                        match self.transaction_state.as_deref() {
                            Some("VOIDED") => AuthipayTransactionStatus::Voided,
                            Some("FAILED") | Some("DECLINED") => AuthipayTransactionStatus::Failed,
                            _ => AuthipayTransactionStatus::Voided, // Default assumption for void requests
                        }
                    }
                }
            }
            _ => {
                // Payment transaction - prioritize transaction_state over transaction_status
                match self.transaction_state.as_deref() {
                    Some("AUTHORIZED") => AuthipayTransactionStatus::Authorized,
                    Some("CAPTURED") => AuthipayTransactionStatus::Captured,
                    Some("VOIDED") => AuthipayTransactionStatus::Voided,
                    Some("DECLINED") | Some("FAILED") => AuthipayTransactionStatus::Failed,
                    _ => {
                        // Fallback to transaction_status with transaction_type context
                        match (
                            self.transaction_type.as_str(),
                            self.transaction_status.as_deref(),
                        ) {
                            // For PREAUTH transactions, "APPROVED" means authorized and awaiting capture
                            ("PREAUTH", Some("APPROVED")) => AuthipayTransactionStatus::Authorized,
                            // For POSTAUTH transactions, "APPROVED" means successfully captured
                            ("POSTAUTH", Some("APPROVED")) => AuthipayTransactionStatus::Captured,
                            // For SALE transactions, "APPROVED" means completed payment
                            ("SALE", Some("APPROVED")) => AuthipayTransactionStatus::Captured,
                            // For VOID transactions, "APPROVED" means successfully voided
                            ("VOID", Some("APPROVED")) => AuthipayTransactionStatus::Voided,
                            // Generic status mappings for other cases
                            (_, Some("APPROVED")) => AuthipayTransactionStatus::Captured,
                            (_, Some("AUTHORIZED")) => AuthipayTransactionStatus::Authorized,
                            (_, Some("DECLINED") | Some("FAILED")) => {
                                AuthipayTransactionStatus::Failed
                            }
                            _ => AuthipayTransactionStatus::Processing,
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentToken {
    reusable: Option<bool>,
    decline_duplicates: Option<bool>,
    brand: Option<String>,
    #[serde(rename = "type")]
    token_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodDetails {
    payment_card: Option<PaymentCardDetails>,
    payment_method_type: Option<String>,
    payment_method_brand: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentCardDetails {
    expiry_date: ExpiryDate,
    bin: String,
    last4: String,
    brand: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Processor {
    reference_number: Option<String>,
    authorization_code: Option<String>,
    response_code: String,
    response_message: String,
    avs_response: Option<AvsResponse>,
    security_code_response: Option<String>,
    tax_refund_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AvsResponse {
    street_match: Option<String>,
    postal_code_match: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, AuthipayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AuthipayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Get gateway response (like Fiserv pattern)
        let gateway_resp = item.response.gateway_response();

        // Store order_id in connector_metadata for void operations (like Fiserv)
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "order_id".to_string(),
            serde_json::Value::String(gateway_resp.transaction_processing_details.order_id.clone()),
        );
        let connector_metadata = Some(serde_json::Value::Object(serde_json::Map::from_iter(
            metadata,
        )));

        Ok(Self {
            status: enums::AttemptStatus::from(gateway_resp.transaction_state.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    gateway_resp
                        .transaction_processing_details
                        .transaction_id
                        .clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    gateway_resp.transaction_processing_details.order_id.clone(),
                ),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// Type definition for CaptureRequest
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayCaptureRequest {
    request_type: &'static str,
    transaction_amount: Amount,
}

impl TryFrom<&AuthipayRouterData<&PaymentsCaptureRouterData>> for AuthipayCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &AuthipayRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            request_type: "PostAuthTransaction",
            transaction_amount: Amount {
                total: item.amount,
                currency: item.router_data.request.currency.to_string(),
                components: None,
            },
        })
    }
}

// Type definition for VoidRequest
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayVoidRequest {
    request_type: &'static str,
}

impl TryFrom<&AuthipayRouterData<&PaymentsCancelRouterData>> for AuthipayVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        _item: &AuthipayRouterData<&PaymentsCancelRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            request_type: "VoidTransaction",
        })
    }
}

// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayRefundRequest {
    request_type: &'static str,
    transaction_amount: Amount,
}

impl<F> TryFrom<&AuthipayRouterData<&RefundsRouterData<F>>> for AuthipayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AuthipayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            request_type: "ReturnTransaction",
            transaction_amount: Amount {
                total: item.amount.to_owned(),
                currency: item.router_data.request.currency.to_string(),
                components: None,
            },
        })
    }
}

// Type definition for Refund Response

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
            //TODO: Review mapping
        }
    }
}

// Reusing the payments response structure for refunds
// because Authipay uses the same endpoint and response format
pub type RefundResponse = AuthipayPaymentsResponse;

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = if item.response.transaction_type == "RETURN" {
            match item.response.transaction_result.as_deref() {
                Some("APPROVED") => RefundStatus::Succeeded,
                Some("DECLINED") | Some("FAILED") => RefundStatus::Failed,
                _ => RefundStatus::Processing,
            }
        } else {
            RefundStatus::Processing
        };

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.ipg_transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(refund_status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = if item.response.transaction_type == "RETURN" {
            match item.response.transaction_result.as_deref() {
                Some("APPROVED") => RefundStatus::Succeeded,
                Some("DECLINED") | Some("FAILED") => RefundStatus::Failed,
                _ => RefundStatus::Processing,
            }
        } else {
            RefundStatus::Processing
        };

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.ipg_transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(refund_status),
            }),
            ..item.data
        })
    }
}

// Error Response structs
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetailItem {
    pub field: String,
    pub message: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails {
    pub code: Option<String>,
    pub message: String,
    pub details: Option<Vec<ErrorDetailItem>>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthipayErrorResponse {
    pub client_request_id: Option<String>,
    pub api_trace_id: Option<String>,
    pub response_type: Option<String>,
    #[serde(rename = "type")]
    pub response_object_type: Option<String>,
    pub error: ErrorDetails,
    pub decline_reason_code: Option<String>,
}

impl From<&AuthipayErrorResponse> for ErrorResponse {
    fn from(item: &AuthipayErrorResponse) -> Self {
        Self {
            status_code: 500, // Default to Internal Server Error, will be overridden by actual HTTP status
            code: item.error.code.clone().unwrap_or_default(),
            message: item.error.message.clone(),
            reason: None,
            attempt_status: None,
            connector_transaction_id: None,
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
            connector_metadata: None,
        }
    }
}
