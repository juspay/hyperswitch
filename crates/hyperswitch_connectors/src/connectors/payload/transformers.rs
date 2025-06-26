use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{
        refunds::{Execute, RSync},
        payments::PaymentMethodToken,
    },
    router_request_types::{ResponseId, PaymentMethodTokenizationData},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsCaptureRouterData, PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{Secret, PeekInterface};
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

// Webhook structures for Payload API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadWebhookBody {
    pub id: String,
    pub event: String,
    pub data: PayloadWebhookData,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadWebhookData {
    pub transaction: Option<PayloadPaymentsResponse>,
    pub refund: Option<RefundResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadWebhookSignature {
    pub timestamp: String,
    pub signature: String,
}

//TODO: Fill the struct with respective fields
pub struct PayloadRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for PayloadRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

// Request structures for Payload API (form-urlencoded)
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PayloadPaymentsRequest {
    amount: StringMinorUnit,
    #[serde(rename = "type")]
    r#type: String,
    status: String,
    payment_method: PayloadPaymentMethod,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PayloadPaymentMethod {
    #[serde(rename = "type")]
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    card: Option<PayloadCard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>, // For tokenized payments
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct PayloadCard {
    card_number: Secret<String>,
    expiry: Secret<String>,
    card_code: Secret<String>,
}

// Capture request structure
#[derive(Debug, Serialize)]
pub struct PayloadCaptureRequest {
    status: String,
}

// Cancel/Void request structure  
#[derive(Debug, Serialize)]
pub struct PayloadCancelRequest {
    status: String,
}

impl TryFrom<&PayloadRouterData<&PaymentsAuthorizeRouterData>> for PayloadPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayloadRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let expiry = format!("{}/{}", 
                    req_card.card_exp_month.peek(),
                    req_card.card_exp_year.peek().get(2..).unwrap_or("00")
                );
                
                let card = PayloadCard {
                    card_number: Secret::new(req_card.card_number.peek().to_string()),
                    expiry: Secret::new(expiry),
                    card_code: req_card.card_cvc,
                };
                
                Ok(Self {
                    amount: item.amount.clone(),
                    r#type: "payment".to_string(),
                    status: "authorized".to_string(),
                    payment_method: PayloadPaymentMethod {
                        r#type: "card".to_string(),
                        card: Some(card),
                        id: None,
                    },
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Capture request transformer
impl TryFrom<&PayloadRouterData<&PaymentsCaptureRouterData>> for PayloadCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: &PayloadRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: "processed".to_string(),
        })
    }
}

// Cancel request transformer (for Void operations)
impl<T> TryFrom<&PayloadRouterData<T>> for PayloadCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: &PayloadRouterData<T>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: "cancelled".to_string(),
        })
    }
}

// Auth Struct for HTTP Basic Authentication
pub struct PayloadAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PayloadAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            ConnectorAuthType::BodyKey { api_key, key1: _ } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse - Updated to match Payload API status values
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PayloadPaymentStatus {
    #[serde(rename = "authorized")]
    Authorized,
    #[serde(rename = "processed")]
    Processed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "pending")]
    #[default]
    Pending,
}

impl From<PayloadPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PayloadPaymentStatus) -> Self {
        match item {
            PayloadPaymentStatus::Authorized => Self::Authorized,
            PayloadPaymentStatus::Processed => Self::Charged,
            PayloadPaymentStatus::Cancelled => Self::Voided,
            PayloadPaymentStatus::Failed => Self::Failure,
            PayloadPaymentStatus::Pending => Self::Pending,
        }
    }
}

// Response structure matching Payload API
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayloadPaymentsResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub amount: i64,
    pub status: PayloadPaymentStatus,
    pub currency: String,
    pub created_at: String,
    pub payment_method: Option<PayloadPaymentMethodResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayloadPaymentMethodResponse {
    id: String,
    #[serde(rename = "type")]
    r#type: String,
    card: Option<PayloadCardResponse>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayloadCardResponse {
    last_four: String,
    brand: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, PayloadPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PayloadPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// REFUND structures matching Payload API
#[derive(Default, Debug, Serialize)]
pub struct PayloadRefundRequest {
    #[serde(rename = "type")]
    r#type: String,
    amount: StringMinorUnit,
    ledger: Vec<PayloadLedgerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadLedgerEntry {
    assoc_transaction_id: String,
}

impl<F> TryFrom<&PayloadRouterData<&RefundsRouterData<F>>> for PayloadRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PayloadRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let connector_transaction_id = item.router_data.request.connector_transaction_id.clone();
        
        Ok(Self {
            r#type: "refund".to_string(),
            amount: item.amount.to_owned(),
            ledger: vec![PayloadLedgerEntry {
                assoc_transaction_id: connector_transaction_id,
            }],
        })
    }
}

// Refund response structures matching Payload API
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PayloadRefundStatus {
    #[serde(rename = "processed")]
    Processed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "pending")]
    #[default]
    Pending,
}

impl From<PayloadRefundStatus> for enums::RefundStatus {
    fn from(item: PayloadRefundStatus) -> Self {
        match item {
            PayloadRefundStatus::Processed => Self::Success,
            PayloadRefundStatus::Failed => Self::Failure,
            PayloadRefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub amount: i64,
    pub status: PayloadRefundStatus,
    pub created_at: String,
    pub ledger: Option<Vec<PayloadLedgerEntry>>,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
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

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PayloadErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

impl PayloadErrorResponse {
    pub fn get_error_message(&self) -> String {
        match &self.reason {
            Some(reason) => format!("{}: {}", self.message, reason),
            None => self.message.clone(),
        }
    }
}

// Status mapping utility functions
pub fn get_payment_status_from_code(status_code: u16, status: &str) -> common_enums::AttemptStatus {
    match (status_code, status) {
        (200..=299, "authorized") => common_enums::AttemptStatus::Authorized,
        (200..=299, "processed") => common_enums::AttemptStatus::Charged,
        (200..=299, "cancelled") => common_enums::AttemptStatus::Voided,
        (200..=299, "pending") => common_enums::AttemptStatus::Pending,
        (400..=499, _) => common_enums::AttemptStatus::Failure,
        (500..=599, _) => common_enums::AttemptStatus::Failure,
        _ => common_enums::AttemptStatus::Failure,
    }
}

pub fn get_refund_status_from_code(status_code: u16, status: &str) -> enums::RefundStatus {
    match (status_code, status) {
        (200..=299, "processed") => enums::RefundStatus::Success,
        (200..=299, "pending") => enums::RefundStatus::Pending,
        (400..=499, _) => enums::RefundStatus::Failure,
        (500..=599, _) => enums::RefundStatus::Failure,
        _ => enums::RefundStatus::Failure,
    }
}

// TOKENIZATION structures matching Payload API
#[derive(Default, Debug, Serialize)]
pub struct PayloadTokenRequest {
    #[serde(rename = "type")]
    r#type: String,
    card: PayloadCard,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PayloadTokenResponse {
    id: String,
    #[serde(rename = "type")]
    r#type: String,
    card: PayloadCardResponse,
    created_at: String,
}

impl TryFrom<&RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>> for PayloadTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let expiry = format!("{}/{}", 
                    req_card.card_exp_month.peek(),
                    req_card.card_exp_year.peek().get(2..).unwrap_or("00")
                );
                
                let card = PayloadCard {
                    card_number: Secret::new(req_card.card_number.peek().to_string()),
                    expiry: Secret::new(expiry),
                    card_code: req_card.card_cvc,
                };
                
                Ok(Self {
                    r#type: "payment_method".to_string(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl TryFrom<ResponseRouterData<PaymentMethodToken, PayloadTokenResponse, PaymentMethodTokenizationData, PaymentsResponseData>>
    for RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<PaymentMethodToken, PayloadTokenResponse, PaymentMethodTokenizationData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.id,
            }),
            ..item.data
        })
    }
}

// Error code mapping for Payload-specific error handling
pub fn get_error_code_mapping(code: &str) -> errors::ConnectorError {
    match code {
        "invalid_card" | "card_declined" | "insufficient_funds" => {
            errors::ConnectorError::FailedAtConnector {
                message: "Payment declined".to_string(),
                code: code.to_string(),
            }
        }
        "invalid_amount" | "amount_too_small" | "amount_too_large" => {
            errors::ConnectorError::InvalidConnectorConfig {
                config: "amount configuration",
            }
        }
        "invalid_currency" => errors::ConnectorError::CurrencyNotSupported {
            message: "Currency not supported".to_string(),
            connector: "Payload",
        },
        "authentication_failed" | "unauthorized" => {
            errors::ConnectorError::FailedToObtainAuthType
        }
        "duplicate_transaction" => errors::ConnectorError::FailedAtConnector {
            message: "Duplicate transaction".to_string(),
            code: code.to_string(),
        },
        "rate_limit_exceeded" => errors::ConnectorError::RequestTimeoutReceived,
        "internal_error" | "server_error" => errors::ConnectorError::ProcessingStepFailed(None),
        _ => errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common_enums::enums::Currency;
    use common_utils::types::MinorUnit;
    use hyperswitch_domain_models::{
        payment_method_data::{Card, CardRedirectData, PaymentMethodData},
        router_data::ConnectorAuthType,
        router_request_types::PaymentsAuthorizeData,
        router_response_types::PaymentsResponseData,
        types::PaymentsAuthorizeRouterData,
    };
    use masking::Secret;

    fn get_test_payment_authorize_data() -> PaymentsAuthorizeData {
        PaymentsAuthorizeData {
            payment_method_data: PaymentMethodData::Card(Box::new(Card {
                card_number: Secret::new("4111111111111111".to_string()),
                card_exp_month: Secret::new("12".to_string()),
                card_exp_year: Secret::new("2025".to_string()),
                card_holder_name: Some(Secret::new("John Doe".to_string())),
                card_cvc: Secret::new("123".to_string()),
                card_issuer: None,
                card_network: None,
                card_type: None,
                card_issuing_country: None,
                bank_code: None,
                nick_name: None,
            })),
            amount: MinorUnit::new(2000),
            minor_amount: MinorUnit::new(2000),
            currency: Currency::USD,
            confirm: true,
            statement_descriptor_suffix: None,
            statement_descriptor: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            capture_method: None,
            browser_info: None,
            order_details: None,
            order_category: None,
            email: None,
            customer_name: None,
            payment_experience: None,
            payment_method_type: None,
            session_token: None,
            enrolled_for_3ds: false,
            related_transaction_id: None,
            router_return_url: None,
            webhook_url: None,
            complete_authorize_url: None,
            customer_id: None,
            surcharge_details: None,
            request_incremental_authorization: None,
            metadata: None,
            authentication_data: None,
            customer_acceptance: None,
            charges: None,
            merchant_order_reference_id: None,
            integrity_object: None,
        }
    }

    #[test]
    fn test_authorize_request_transformation() {
        let auth_data = get_test_payment_authorize_data();
        let router_data = PaymentsAuthorizeRouterData {
            flow: std::marker::PhantomData,
            merchant_id: "test_merchant".to_string(),
            customer_id: Some("test_customer".to_string()),
            connector_customer: None,
            payment_id: "test_payment".to_string(),
            attempt_id: "test_attempt".to_string(),
            status: common_enums::AttemptStatus::Pending,
            payment_method: common_enums::PaymentMethod::Card,
            connector_auth_type: ConnectorAuthType::HeaderKey {
                api_key: Secret::new("test_api_key".to_string()),
            },
            description: None,
            return_url: None,
            address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
            auth_type: common_enums::AuthenticationType::NoThreeDs,
            connector_meta_data: None,
            connector_wallets_details: None,
            request: auth_data,
            response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            connector_api_version: None,
            connector_http_status_code: None,
            external_latency: None,
            connector_request_reference_id: "test_ref".to_string(),
            test_mode: None,
            payment_method_balance: None,
            connector_response: None,
            integrity_check: Ok(()),
        };

        let amount = common_utils::types::StringMinorUnit::new(2000);
        let payload_router_data = PayloadRouterData::from((amount, &router_data));
        let result = PayloadPaymentsRequest::try_from(&payload_router_data);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.r#type, "payment");
        assert_eq!(request.status, "authorized");
        assert_eq!(request.payment_method.r#type, "card");
        assert!(request.payment_method.card.is_some());
    }

    #[test]
    fn test_payment_status_mapping() {
        assert_eq!(
            common_enums::AttemptStatus::from(PayloadPaymentStatus::Authorized),
            common_enums::AttemptStatus::Authorized
        );
        assert_eq!(
            common_enums::AttemptStatus::from(PayloadPaymentStatus::Processed),
            common_enums::AttemptStatus::Charged
        );
        assert_eq!(
            common_enums::AttemptStatus::from(PayloadPaymentStatus::Cancelled),
            common_enums::AttemptStatus::Voided
        );
        assert_eq!(
            common_enums::AttemptStatus::from(PayloadPaymentStatus::Failed),
            common_enums::AttemptStatus::Failure
        );
        assert_eq!(
            common_enums::AttemptStatus::from(PayloadPaymentStatus::Pending),
            common_enums::AttemptStatus::Pending
        );
    }

    #[test]
    fn test_refund_status_mapping() {
        assert_eq!(
            enums::RefundStatus::from(PayloadRefundStatus::Processed),
            enums::RefundStatus::Success
        );
        assert_eq!(
            enums::RefundStatus::from(PayloadRefundStatus::Failed),
            enums::RefundStatus::Failure
        );
        assert_eq!(
            enums::RefundStatus::from(PayloadRefundStatus::Pending),
            enums::RefundStatus::Pending
        );
    }

    #[test]
    fn test_error_code_mapping() {
        // Test payment declined errors
        let error = get_error_code_mapping("invalid_card");
        match error {
            errors::ConnectorError::FailedAtConnector { message, code } => {
                assert_eq!(message, "Payment declined");
                assert_eq!(code, "invalid_card");
            }
            _ => panic!("Expected FailedAtConnector error"),
        }

        // Test currency not supported
        let error = get_error_code_mapping("invalid_currency");
        match error {
            errors::ConnectorError::CurrencyNotSupported { message, connector } => {
                assert_eq!(message, "Currency not supported");
                assert_eq!(connector, "Payload");
            }
            _ => panic!("Expected CurrencyNotSupported error"),
        }

        // Test authentication error
        let error = get_error_code_mapping("authentication_failed");
        assert_eq!(error, errors::ConnectorError::FailedToObtainAuthType);
    }

    #[test]
    fn test_payload_auth_type_from_header_key() {
        let auth_type = ConnectorAuthType::HeaderKey {
            api_key: Secret::new("test_key".to_string()),
        };
        let result = PayloadAuthType::try_from(&auth_type);
        assert!(result.is_ok());
    }

    #[test]
    fn test_payload_auth_type_from_body_key() {
        let auth_type = ConnectorAuthType::BodyKey {
            api_key: Secret::new("test_key".to_string()),
            key1: "test_key1".to_string(),
        };
        let result = PayloadAuthType::try_from(&auth_type);
        assert!(result.is_ok());
    }

    #[test]
    fn test_capture_request_transformation() {
        use hyperswitch_domain_models::{
            router_request_types::PaymentsCaptureData,
            types::PaymentsCaptureRouterData,
        };

        let capture_data = PaymentsCaptureData {
            amount_to_capture: MinorUnit::new(1000),
            minor_amount_to_capture: MinorUnit::new(1000),
            currency: Currency::USD,
            connector_transaction_id: "test_txn_id".to_string(),
            payment_amount: MinorUnit::new(2000),
            minor_payment_amount: MinorUnit::new(2000),
            connector_meta: None,
            multiple_capture_data: None,
            browser_info: None,
            metadata: None,
            integrity_object: None,
        };

        let router_data = PaymentsCaptureRouterData {
            flow: std::marker::PhantomData,
            merchant_id: "test_merchant".to_string(),
            customer_id: Some("test_customer".to_string()),
            connector_customer: None,
            payment_id: "test_payment".to_string(),
            attempt_id: "test_attempt".to_string(),
            status: common_enums::AttemptStatus::Pending,
            payment_method: common_enums::PaymentMethod::Card,
            connector_auth_type: ConnectorAuthType::HeaderKey {
                api_key: Secret::new("test_api_key".to_string()),
            },
            description: None,
            return_url: None,
            address: hyperswitch_domain_models::payment_address::PaymentAddress::default(),
            auth_type: common_enums::AuthenticationType::NoThreeDs,
            connector_meta_data: None,
            connector_wallets_details: None,
            request: capture_data,
            response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            connector_api_version: None,
            connector_http_status_code: None,
            external_latency: None,
            connector_request_reference_id: "test_ref".to_string(),
            test_mode: None,
            payment_method_balance: None,
            connector_response: None,
            integrity_check: Ok(()),
        };

        let amount = common_utils::types::StringMinorUnit::new(1000);
        let payload_router_data = PayloadRouterData::from((amount, &router_data));
        let result = PayloadCaptureRequest::try_from(&payload_router_data);

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.status, "processed");
    }
}
