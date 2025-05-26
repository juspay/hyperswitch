use common_enums::enums;
use common_utils::types::StringMinorUnit;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentMethodTokenizationData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, PaymentsAuthorizeRequestData, PaymentsSyncRequestData, RouterData as RouterDataTrait},
};

// Step 3: Core enums and types

/// Maxpay transaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MaxpayTransactionType {
    Auth,
    Auth3d,
    Sale,
    Sale3d,
    Settle,
    Check,
    Tokenize,
}

/// Maxpay transaction status
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MaxpayStatus {
    Success,
    Decline,
    Error,
    #[default]
    #[serde(other)]
    Unknown,
}

/// Maxpay authentication credentials
#[derive(Debug, Clone)]
pub struct MaxpayAuth {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for MaxpayAuth {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                merchant_account: api_key.clone(),
                merchant_password: key1.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// Step 4: Authorization request/response types

/// Authorization request for AUTH and AUTH3D transactions
#[derive(Debug, Serialize)]
pub struct MaxpayAuthRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    #[serde(rename = "transactionType")]
    pub transaction_type: MaxpayTransactionType,
    pub amount: f64,
    pub currency: String, // ISO 4217 alpha-3
    pub card_number: Secret<String>,
    pub card_expiry: Secret<String>, // MM/YYYY format
    pub card_cvv: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_country: Option<String>, // ISO 3166-1 alpha-2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_zip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_sku: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_transaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bill_token: Option<String>, // For tokenized payments
}

/// Authorization response
#[derive(Debug, Serialize, Deserialize)]
pub struct MaxpayAuthResponse {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    pub reference: String,
    pub status: MaxpayStatus,
    pub code: i32,
    #[serde(rename = "redirectUrl")]
    pub redirect_url: Option<String>,
    #[serde(rename = "billToken")]
    pub bill_token: Option<String>,
    pub message: Option<String>,
}

// Step 5: Capture and sync types

/// Capture request for SETTLE transaction
#[derive(Debug, Serialize)]
pub struct MaxpayCaptureRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    #[serde(rename = "transactionType")]
    pub transaction_type: MaxpayTransactionType, // SETTLE
    pub reference: String,
}

/// Sync request for CHECK transaction
#[derive(Debug, Serialize)]
pub struct MaxpaySyncRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    #[serde(rename = "transactionType")]
    pub transaction_type: MaxpayTransactionType, // CHECK
    pub reference: String,
}

/// Response for capture and sync operations
#[derive(Debug, Serialize, Deserialize)]
pub struct MaxpayCaptureResponse {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    pub reference: String,
    pub status: MaxpayStatus,
    pub code: i32,
    pub message: Option<String>,
}

/// Response for sync operations (same structure as capture)
pub type MaxpaySyncResponse = MaxpayCaptureResponse;

// Step 6: Refund and tokenization types

/// Refund request
#[derive(Debug, Serialize)]
pub struct MaxpayRefundRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    pub reference: String,
    pub amount: f64,
}

/// Refund response
#[derive(Debug, Serialize, Deserialize)]
pub struct MaxpayRefundResponse {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    pub reference: String,
    pub status: MaxpayStatus,
    pub code: i32,
    pub message: Option<String>,
}

/// Tokenization request
#[derive(Debug, Serialize)]
pub struct MaxpayTokenizeRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    #[serde(rename = "transactionType")]
    pub transaction_type: MaxpayTransactionType, // TOKENIZE
    pub card_number: Secret<String>,
    pub card_expiry: Secret<String>, // MM/YYYY format
    pub card_cvv: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_phone: Option<String>,
}

/// Tokenization response
#[derive(Debug, Serialize, Deserialize)]
pub struct MaxpayTokenizeResponse {
    #[serde(rename = "billToken")]
    pub bill_token: String,
    pub status: MaxpayStatus,
    pub code: i32,
    pub message: Option<String>,
}

// Step 7: Webhook types

/// Webhook callback v1.0 (form-urlencoded)
#[derive(Debug, Serialize, Deserialize)]
pub struct MaxpayWebhookV1 {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    pub reference: String,
    pub status: MaxpayStatus,
    pub code: i32,
    #[serde(rename = "checkSum")]
    pub check_sum: String,
}

/// Webhook callback v2.0 (JSON)
#[derive(Debug, Serialize, Deserialize)]
pub struct MaxpayWebhookV2 {
    #[serde(rename = "uniqueTransactionId")]
    pub unique_transaction_id: String,
    pub reference: String,
    pub status: MaxpayStatus,
    pub code: i32,
}

/// Enum to handle both webhook versions
#[derive(Debug)]
pub enum MaxpayWebhook {
    V1(MaxpayWebhookV1),
    V2(MaxpayWebhookV2),
}

// Router data wrapper
pub struct MaxpayRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for MaxpayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// Add missing MaxpayAuthType
pub struct MaxpayAuthType {
    pub api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for MaxpayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, .. } => Ok(Self {
                api_key: api_key.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// Actual payment request/response types that match the expected structure
pub type MaxpayPaymentsRequest = MaxpayAuthRequest;
pub type MaxpayPaymentsResponse = MaxpayAuthResponse;

// Implementation for converting router data to payment request
impl<'a> TryFrom<&MaxpayRouterData<&'a PaymentsAuthorizeRouterData>> for MaxpayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &MaxpayRouterData<&'a PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = MaxpayAuth::try_from(&item.router_data.connector_auth_type)?;
        let payment_data = &item.router_data.request;
        
        // Extract card details
        let card = match &payment_data.payment_method_data {
            PaymentMethodData::Card(card_data) => card_data,
            _ => return Err(errors::ConnectorError::NotSupported {
                message: "Only card payments are supported".to_string(),
                connector: "Maxpay",
            }.into()),
        };

        // Format card expiry as MM/YYYY
        let card_expiry = format!("{:02}/{}", 
            card.card_exp_month.peek(), 
            card.card_exp_year.peek()
        );

        // Convert amount to f64
        // Maxpay expects amounts in major units (e.g., 10.50 for $10.50)
        // Serialize StringMinorUnit to get the string value
        let amount_str = serde_json::to_string(&item.amount)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?
            .trim_matches('"')
            .to_string();
        let amount_i64: i64 = amount_str.parse::<i64>()
            .map_err(|_| errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to parse amount as i64")?;
        // Convert to major units as a string
        let amount_str = utils::to_currency_base_unit(amount_i64, payment_data.currency)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        // Parse to f64
        let amount: f64 = amount_str.parse::<f64>()
            .map_err(|_| errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to parse amount as f64")?;

        // Determine transaction type based on capture method and 3DS requirements
        // If we have both callback_url and redirect_url, we should use 3DS flow
        let use_3ds = payment_data.webhook_url.is_some() && payment_data.complete_authorize_url.is_some();
        
        let transaction_type = match (payment_data.capture_method, use_3ds) {
            (Some(common_enums::CaptureMethod::Manual), true) => MaxpayTransactionType::Auth3d,
            (Some(common_enums::CaptureMethod::Manual), false) => MaxpayTransactionType::Auth,
            (_, true) => MaxpayTransactionType::Sale3d,
            (_, false) => MaxpayTransactionType::Sale,
        };

        // Get browser info and billing address
        let browser_info = item.router_data.request.get_browser_info()?;
        let billing_address = item.router_data.get_billing_address()?;

        Ok(Self {
            merchant_account: auth.merchant_account,
            merchant_password: auth.merchant_password,
            transaction_type,
            amount,
            currency: payment_data.currency.to_string(),
            card_number: Secret::new(card.card_number.peek().to_string()),
            card_expiry: Secret::new(card_expiry),
            card_cvv: card.card_cvc.clone(),
            callback_url: payment_data.webhook_url.clone(),
            redirect_url: payment_data.complete_authorize_url.clone(),
            user_ip: browser_info.ip_address.map(|ip| ip.to_string()),
            user_email: payment_data.email.as_ref().map(|email| email.peek().to_string()),
            user_phone: item.router_data.get_optional_billing_phone_number()
                .map(|phone| phone.expose()),
            user_first_name: billing_address.first_name.as_ref()
                .map(|name| name.peek().to_string()),
            user_last_name: billing_address.last_name.as_ref()
                .map(|name| name.peek().to_string()),
            user_address: billing_address.line1.as_ref()
                .map(|addr| addr.peek().to_string()),
            user_city: billing_address.city.clone(),
            user_country: billing_address.country.as_ref()
                .map(|country| country.to_string()),
            user_state: billing_address.state.as_ref()
                .map(|state| state.peek().to_string()),
            user_zip: billing_address.zip.as_ref()
                .map(|zip| zip.peek().to_string()),
            product_name: None,
            product_description: payment_data.order_details.as_ref()
                .and_then(|od| od.first())
                .map(|o| o.product_name.clone()),
            product_sku: None,
            product_category: None,
            order_id: Some(item.router_data.connector_request_reference_id.clone()),
            merchant_transaction_id: Some(item.router_data.payment_id.clone()),
            bill_token: None,
        })
    }
}

impl From<MaxpayStatus> for common_enums::AttemptStatus {
    fn from(item: MaxpayStatus) -> Self {
        match item {
            MaxpayStatus::Success => Self::Charged,
            MaxpayStatus::Decline => Self::Failure,
            MaxpayStatus::Error => Self::Failure,
            MaxpayStatus::Unknown => Self::Pending,
        }
    }
}

/// Custom status mapping for 3DS flows
pub fn get_attempt_status_for_3ds(status: &MaxpayStatus, redirect_url: &Option<String>) -> common_enums::AttemptStatus {
    match (status, redirect_url) {
        // If we have a redirect URL, the payment needs authentication
        (_, Some(_)) => common_enums::AttemptStatus::AuthenticationPending,
        // Otherwise, use the standard status mapping
        _ => common_enums::AttemptStatus::from(status.clone()),
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, MaxpayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MaxpayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Use 3DS-aware status mapping
        let status = get_attempt_status_for_3ds(&item.response.status, &item.response.redirect_url);
        
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.transaction_id),
                redirection_data: Box::new(item.response.redirect_url.map(|url| {
                    hyperswitch_domain_models::router_response_types::RedirectForm::Form {
                        endpoint: url,
                        method: common_utils::request::Method::Get,
                        form_fields: std::collections::HashMap::new(),
                    }
                })),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.reference),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// Use the actual refund response type
pub type RefundResponse = MaxpayRefundResponse;

// Implementation for converting router data to refund request
impl<F> TryFrom<&MaxpayRouterData<&RefundsRouterData<F>>> for MaxpayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &MaxpayRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let auth = MaxpayAuth::try_from(&item.router_data.connector_auth_type)?;
        
        // Convert amount to f64
        // Maxpay expects amounts in major units (e.g., 10.50 for $10.50)
        // Serialize StringMinorUnit to get the string value
        let amount_str = serde_json::to_string(&item.amount)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?
            .trim_matches('"')
            .to_string();
        let amount_i64: i64 = amount_str.parse::<i64>()
            .map_err(|_| errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to parse amount as i64")?;
        // Convert to major units as a string
        let amount_str = utils::to_currency_base_unit(amount_i64, item.router_data.request.currency)
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        // Parse to f64
        let amount: f64 = amount_str.parse::<f64>()
            .map_err(|_| errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to parse amount as f64")?;

        // Get the reference from connector_transaction_id
        let reference = item.router_data.request.connector_transaction_id.clone();

        Ok(Self {
            merchant_account: auth.merchant_account,
            merchant_password: auth.merchant_password,
            reference,
            amount,
        })
    }
}

impl From<MaxpayStatus> for enums::RefundStatus {
    fn from(item: MaxpayStatus) -> Self {
        match item {
            MaxpayStatus::Success => Self::Success,
            MaxpayStatus::Decline | MaxpayStatus::Error => Self::Failure,
            MaxpayStatus::Unknown => Self::Pending,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id.to_string(),
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
                connector_refund_id: item.response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct MaxpayErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

// Implementation for capture request
impl TryFrom<&PaymentsCaptureRouterData> for MaxpayCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &PaymentsCaptureRouterData,
    ) -> Result<Self, Self::Error> {
        let auth = MaxpayAuth::try_from(&item.connector_auth_type)?;
        
        // Get the reference from the previous authorization
        let reference = item.request.connector_transaction_id.clone();

        Ok(Self {
            merchant_account: auth.merchant_account,
            merchant_password: auth.merchant_password,
            transaction_type: MaxpayTransactionType::Settle,
            reference,
        })
    }
}

// Implementation for sync request
impl TryFrom<&PaymentsSyncRouterData> for MaxpaySyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &PaymentsSyncRouterData,
    ) -> Result<Self, Self::Error> {
        let auth = MaxpayAuth::try_from(&item.connector_auth_type)?;
        
        // Get the reference from connector_transaction_id
        let reference = item.request.get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?
            .to_string();

        Ok(Self {
            merchant_account: auth.merchant_account,
            merchant_password: auth.merchant_password,
            transaction_type: MaxpayTransactionType::Check,
            reference,
        })
    }
}

// Implementation for tokenization request
impl TryFrom<&RouterData<hyperswitch_domain_models::router_flow_types::payments::PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>> 
    for MaxpayTokenizeRequest 
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &RouterData<hyperswitch_domain_models::router_flow_types::payments::PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let auth = MaxpayAuth::try_from(&item.connector_auth_type)?;
        
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(card_data) => {
                // Format card expiry as MM/YYYY
                let card_expiry = format!("{:02}/{}", 
                    card_data.card_exp_month.peek(), 
                    card_data.card_exp_year.peek()
                );

                Ok(Self {
                    merchant_account: auth.merchant_account,
                    merchant_password: auth.merchant_password,
                    transaction_type: MaxpayTransactionType::Tokenize,
                    card_number: Secret::new(card_data.card_number.peek().to_string()),
                    card_expiry: Secret::new(card_expiry),
                    card_cvv: card_data.card_cvc.clone(),
                    user_email: None,
                    user_phone: None,
                })
            }
            _ => Err(errors::ConnectorError::NotSupported {
                message: "Only card tokenization is supported".to_string(),
                connector: "Maxpay",
            }.into()),
        }
    }
}

// Implementation for tokenization response
impl<F> TryFrom<ResponseRouterData<F, MaxpayTokenizeResponse, PaymentMethodTokenizationData, PaymentsResponseData>>
    for RouterData<F, PaymentMethodTokenizationData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MaxpayTokenizeResponse, PaymentMethodTokenizationData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = if item.response.status == MaxpayStatus::Success {
            common_enums::AttemptStatus::Charged
        } else {
            common_enums::AttemptStatus::Failure
        };

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.bill_token,
            }),
            ..item.data
        })
    }
}
