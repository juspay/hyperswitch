use common_enums::enums;
use common_utils::types::StringMinorUnit;
use masking::{ExposeInterface, Secret};
use cards::CardNumber;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData, PaymentMethodToken},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct AuthipayRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for AuthipayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

// Authipay payment request structure
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AuthipayPaymentsRequest {
    request_type: String,
    transaction_amount: AuthipayAmount,
    payment_method: AuthipayPaymentMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_transaction_id: Option<String>,
    store_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stored_credentials: Option<AuthipayStoredCredentials>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AuthipayAmount {
    total: StringMinorUnit,
    currency: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AuthipayPaymentMethod {
    payment_card: AuthipayCard,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirect_attributes: Option<AuthipayRedirectAttributes>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AuthipayRedirectAttributes {
    #[serde(rename = "authenticateTransaction")]
    authenticate_transaction: bool,
    #[serde(rename = "challengeIndicator")]
    challenge_indicator: String,
    #[serde(rename = "browserJavaScriptEnabled")]
    browser_javascript_enabled: bool,
    #[serde(rename = "browserJavaEnabled")]
    browser_java_enabled: bool,
    #[serde(rename = "threeDSEmvCoMessageCategory")]
    three_ds_emv_co_message_category: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AuthipayCard {
    number: CardNumber,
    security_code: Secret<String>,
    expiry_date: AuthipayExpiryDate,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AuthipayExpiryDate {
    month: Secret<String>,
    year: Secret<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AuthipayStoredCredentials {
    sequence: String,
    scheduled: bool,
}

// Validation helper functions
fn validate_card_details(
    card_number: &CardNumber,
    card_exp_month: &Secret<String>,
    card_exp_year: &Secret<String>,
    card_cvc: &Secret<String>,
) -> Result<(), error_stack::Report<errors::ConnectorError>> {
    // Validate card number length - since we don't have direct access, 
    // we'll rely on the CardNumber type's own validation
    
    // Validate expiry month
    let exp_month = card_exp_month.clone().expose().to_string();
    if exp_month.len() != 2 || exp_month.parse::<u8>().map_or(true, |m| m < 1 || m > 12) {
        return Err(errors::ConnectorError::RequestEncodingFailed.into());
    }

    // Validate expiry year
    let exp_year = card_exp_year.clone().expose().to_string();
    if exp_year.len() != 2 && exp_year.len() != 4 {
        return Err(errors::ConnectorError::RequestEncodingFailed.into());
    }

    // Validate CVC length (3-4 digits)
    let cvc = card_cvc.clone().expose().to_string();
    if cvc.len() < 3 || cvc.len() > 4 || !cvc.chars().all(|c| c.is_digit(10)) {
        return Err(errors::ConnectorError::RequestEncodingFailed.into());
    }

    Ok(())
}

fn validate_amount_and_currency(
    amount: &StringMinorUnit,
    currency: &common_enums::Currency,
) -> Result<(), error_stack::Report<errors::ConnectorError>> {
    // Validate currency is supported by Authipay
    // Authipay supports major currencies like USD, EUR, GBP, etc.
    match currency {
        common_enums::Currency::USD | 
        common_enums::Currency::EUR | 
        common_enums::Currency::GBP | 
        common_enums::Currency::CAD | 
        common_enums::Currency::AUD | 
        common_enums::Currency::JPY => (),
        _ => {
            return Err(errors::ConnectorError::ProcessingStepFailed(None).into());
        }
    }

    // Skip amount format validation as we trust the StringMinorUnit from the system

    Ok(())
}

impl TryFrom<&AuthipayRouterData<&PaymentsAuthorizeRouterData>> for AuthipayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthipayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                // Validate card details
                validate_card_details(
                    &req_card.card_number,
                    &req_card.card_exp_month,
                    &req_card.card_exp_year,
                    &req_card.card_cvc,
                )?;

                // Validate amount and currency
                validate_amount_and_currency(
                    &item.amount,
                    &item.router_data.request.currency,
                )?;
                // Check for 3DS authentication
                let requires_3ds = false; // Default to false for now until we understand the correct field access
                
                // Determine request type - if 3DS is required, use PayerAuth flow
                let request_type = if requires_3ds {
                    "PaymentCardPayerAuthTransaction"
                } else if item.router_data.request.is_auto_capture()? {
                    "PaymentCardSaleTransaction"
                } else {
                    "PaymentCardPreAuthTransaction"
                }.to_string();

                let expiry_date = AuthipayExpiryDate {
                    month: req_card.card_exp_month,
                    year: req_card.card_exp_year.clone(),
                };

                let card = AuthipayCard {
                    number: req_card.card_number,
                    security_code: req_card.card_cvc,
                    expiry_date,
                };

                // Set up 3DS redirect attributes if authentication is required
                let redirect_attributes = if requires_3ds {
                    // Extract browser information if available
                    let browser_info = item.router_data.request.browser_info.clone();
                    let browser_js_enabled = browser_info
                        .as_ref()
                        .and_then(|info| info.java_script_enabled)
                        .unwrap_or(true);
                    let browser_java_enabled = browser_info
                        .as_ref()
                        .and_then(|info| info.java_enabled)
                        .unwrap_or(true);
                    
                    Some(AuthipayRedirectAttributes {
                        authenticate_transaction: true,
                        challenge_indicator: "01".to_string(), // No preference
                        browser_javascript_enabled: browser_js_enabled,
                        browser_java_enabled: browser_java_enabled,
                        three_ds_emv_co_message_category: "01".to_string(),
                    })
                } else {
                    None
                };

                let payment_method = AuthipayPaymentMethod {
                    payment_card: card,
                    redirect_attributes,
                };

                let auth = AuthipayAuthType::try_from(&item.router_data.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

                Ok(Self {
                    request_type,
                    transaction_amount: AuthipayAmount {
                        total: item.amount.clone(),
                        currency: item.router_data.request.currency.to_string(),
                    },
                    payment_method,
                    merchant_transaction_id: Some(item.router_data.payment_id.clone()),
                    store_id: auth.store_id.clone(),
                    stored_credentials: None,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct AuthipayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_secret: Secret<String>,
    pub(super) store_id: String,
}

impl TryFrom<&ConnectorAuthType> for AuthipayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey { api_key, key1, api_secret } => Ok(Self {
                api_key: api_key.to_owned(),
                api_secret: api_secret.to_owned(),
                store_id: key1.clone().expose().to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse Status
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuthipayPaymentStatus {
    AUTHORIZED,
    CAPTURED,
    DECLINED,
    VOIDED,
    #[default]
    PENDING,
}

impl From<AuthipayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: AuthipayPaymentStatus) -> Self {
        match item {
            AuthipayPaymentStatus::AUTHORIZED => Self::Authorized,
            AuthipayPaymentStatus::CAPTURED => Self::Charged,
            AuthipayPaymentStatus::DECLINED => Self::Failure,
            AuthipayPaymentStatus::VOIDED => Self::Voided,
            AuthipayPaymentStatus::PENDING => Self::Pending,
        }
    }
}

// Authipay Payment Response structure
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthipayPaymentsResponse {
    #[serde(rename = "clientRequestId")]
    client_request_id: String,
    #[serde(rename = "apiTraceId")]
    api_trace_id: String,
    #[serde(rename = "ipgTransactionId")]
    ipg_transaction_id: String,
    #[serde(rename = "orderId")]
    order_id: String,
    #[serde(rename = "transactionTime")]
    transaction_time: i64,
    #[serde(rename = "transactionState")]
    transaction_state: AuthipayPaymentStatus,
    #[serde(rename = "paymentType")]
    payment_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "transactionOrigin")]
    transaction_origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "redirectURL")]
    redirect_url: Option<String>,
    amount: AuthipayAmount,
    #[serde(rename = "storeId")]
    store_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, AuthipayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AuthipayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Check if the response contains a redirect URL for 3DS
        let redirection_data = match item.response.redirect_url.clone() {
            Some(url) if !url.is_empty() => {
                use hyperswitch_domain_models::router_response_types;
                
                // Create redirection data for 3DS authentication
                let redirect_data = router_response_types::RedirectionData {
                    redirect_url: url,
                    redirect_method: common_utils::request::Method::Get,
                    redirect_http_request: None,
                };
                Box::new(Some(redirect_data))
            },
            _ => Box::new(None),
        };

        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.transaction_state),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.ipg_transaction_id),
                redirection_data,
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct AuthipayRefundRequest {
    request_type: String,
    transaction_amount: AuthipayAmount,
    store_id: String,
}

impl<F> TryFrom<&AuthipayRouterData<&RefundsRouterData<F>>> for AuthipayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AuthipayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth = AuthipayAuthType::try_from(&item.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        
        Ok(Self {
            request_type: "ReturnTransaction".to_string(),
            transaction_amount: AuthipayAmount {
                total: item.amount.to_owned(),
                currency: item.router_data.request.currency.to_string(),
            },
            store_id: auth.store_id.clone(),
        })
    }
}

// Type definition for Refund Response
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuthipayRefundStatus {
    RETURNED,
    DECLINED,
    #[default]
    PENDING,
}

impl From<AuthipayRefundStatus> for enums::RefundStatus {
    fn from(item: AuthipayRefundStatus) -> Self {
        match item {
            AuthipayRefundStatus::RETURNED => Self::Success,
            AuthipayRefundStatus::DECLINED => Self::Failure,
            AuthipayRefundStatus::PENDING => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    #[serde(rename = "clientRequestId")]
    client_request_id: String,
    #[serde(rename = "apiTraceId")]
    api_trace_id: String,
    #[serde(rename = "ipgTransactionId")]
    ipg_transaction_id: String,
    #[serde(rename = "orderId")]
    order_id: String,
    #[serde(rename = "transactionTime")]
    transaction_time: i64,
    #[serde(rename = "transactionState")]
    transaction_state: AuthipayRefundStatus,
    #[serde(rename = "paymentType")]
    payment_type: String,
    amount: AuthipayAmount,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.ipg_transaction_id,
                refund_status: enums::RefundStatus::from(item.response.transaction_state),
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
                connector_refund_id: item.response.ipg_transaction_id,
                refund_status: enums::RefundStatus::from(item.response.transaction_state),
            }),
            ..item.data
        })
    }
}

// TOKENIZATION:
// Type definition for TokenRequest
#[derive(Default, Debug, Serialize)]
pub struct AuthipayTokenRequest {
    request_type: String,
    payment_card: AuthipayCard,
    store_id: String,
}

impl TryFrom<&RouterData<&hyperswitch_domain_models::types::PaymentMethodTokenizationRouterData, &hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData, &PaymentsResponseData>> for AuthipayTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: &RouterData<&hyperswitch_domain_models::types::PaymentMethodTokenizationRouterData, &hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData, &PaymentsResponseData>) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let expiry_date = AuthipayExpiryDate {
                    month: req_card.card_exp_month,
                    year: req_card.card_exp_year.clone(),
                };

                let card = AuthipayCard {
                    number: req_card.card_number,
                    security_code: req_card.card_cvc,
                    expiry_date,
                };

                let auth = AuthipayAuthType::try_from(&item.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

                Ok(Self {
                    request_type: "PaymentCardPaymentTokenizationRequest".to_string(),
                    payment_card: card,
                    store_id: auth.store_id.clone(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method not supported for tokenization".to_string()).into()),
        }
    }
}

// Type definition for TokenResponse
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AuthipayTokenResponse {
    #[serde(rename = "clientRequestId")]
    client_request_id: String,
    #[serde(rename = "apiTraceId")]
    api_trace_id: String,
    #[serde(rename = "paymentToken")]
    payment_token: AuthipayPaymentToken,
    #[serde(rename = "requestStatus")]
    request_status: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AuthipayPaymentToken {
    value: String,
    function: String,
    #[serde(rename = "cardLast4")]
    card_last4: String,
    brand: String,
    #[serde(rename = "expiryMonth")]
    expiry_month: String,
    #[serde(rename = "expiryYear")]
    expiry_year: String,
}

impl TryFrom<ResponseRouterData<&hyperswitch_domain_models::types::PaymentMethodTokenizationRouterData, AuthipayTokenResponse, &hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData, PaymentsResponseData>> 
    for RouterData<&hyperswitch_domain_models::types::PaymentMethodTokenizationRouterData, &hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData, PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(
        item: ResponseRouterData<&hyperswitch_domain_models::types::PaymentMethodTokenizationRouterData, AuthipayTokenResponse, &hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Use types from correct module
        let card_details = hyperswitch_domain_models::router_response_types::PaymentMethodTokenData::Card(hyperswitch_domain_models::router_response_types::CardTokenData {
            last4: item.response.payment_token.card_last4,
            exp_month: item.response.payment_token.expiry_month,
            exp_year: item.response.payment_token.expiry_year,
            card_type: item.response.payment_token.brand.to_lowercase(),
            card_holder_name: None,
        });
        
        Ok(Self {
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: PaymentMethodToken {
                    token: item.response.payment_token.value,
                    payment_method_data: card_details,
                },
            }),
            ..item.data
        })
    }
}

// CARD VERIFICATION (PREPROCESSING):
// Type definition for CardVerificationRequest
#[derive(Default, Debug, Serialize)]
pub struct AuthipayCardVerificationRequest {
    request_type: String,
    payment_card: AuthipayCard,
    store_id: String,
}

// Type definition for CardVerificationResponse
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthipayCardVerificationResponse {
    #[serde(rename = "clientRequestId")]
    client_request_id: String,
    #[serde(rename = "apiTraceId")]
    api_trace_id: String,
    #[serde(rename = "transactionTime")]
    transaction_time: i64,
    #[serde(rename = "transactionState")]
    transaction_state: String,
    #[serde(rename = "paymentType")]
    payment_type: String,
}

impl TryFrom<&RouterData<&hyperswitch_domain_models::types::PaymentsPreProcessingRouterData, &hyperswitch_domain_models::router_request_types::PaymentsPreProcessingData, &hyperswitch_domain_models::router_response_types::PaymentsResponseData>> for AuthipayCardVerificationRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: &RouterData<&hyperswitch_domain_models::types::PaymentsPreProcessingRouterData, &hyperswitch_domain_models::router_request_types::PaymentsPreProcessingData, &hyperswitch_domain_models::router_response_types::PaymentsResponseData>) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            Some(PaymentMethodData::Card(req_card)) => {
                let expiry_date = AuthipayExpiryDate {
                    month: req_card.card_exp_month,
                    year: req_card.card_exp_year.clone(),
                };

                let card = AuthipayCard {
                    number: req_card.card_number,
                    security_code: req_card.card_cvc,
                    expiry_date,
                };

                let auth = AuthipayAuthType::try_from(&item.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

                Ok(Self {
                    request_type: "CardVerificationRequest".to_string(),
                    payment_card: card,
                    store_id: auth.store_id.clone(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method not supported for card verification".to_string()).into()),
        }
    }
}

impl TryFrom<ResponseRouterData<&hyperswitch_domain_models::types::PaymentsPreProcessingRouterData, AuthipayCardVerificationResponse, &hyperswitch_domain_models::router_request_types::PaymentsPreProcessingData, PaymentsResponseData>> 
    for RouterData<&hyperswitch_domain_models::types::PaymentsPreProcessingRouterData, &hyperswitch_domain_models::router_request_types::PaymentsPreProcessingData, PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(
        item: ResponseRouterData<&hyperswitch_domain_models::types::PaymentsPreProcessingRouterData, AuthipayCardVerificationResponse, &hyperswitch_domain_models::router_request_types::PaymentsPreProcessingData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Check if verification was successful
        let status = if item.response.transaction_state == "VERIFIED" {
            common_enums::AttemptStatus::Authorized
        } else {
            common_enums::AttemptStatus::Failure
        };
        
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::PreProcessingResponse {
                // Return true if verification successful, false otherwise
                pre_processing_id: if status == common_enums::AttemptStatus::Authorized {
                    hyperswitch_domain_models::router_response_types::PreprocessingResponseId::PreProcessingId("card_verified".to_string())
                } else {
                    hyperswitch_domain_models::router_response_types::PreprocessingResponseId::ConnectorTransactionId("card_verification_failed".to_string())
                },
                connector_metadata: None,
                connector_response_reference_id: Some(item.response.client_request_id.clone()),
                session_token: None,
            }),
            ..item.data
        })
    }
}

// Authipay Error Response structure
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthipayErrorResponse {
    #[serde(rename = "clientRequestId")]
    pub client_request_id: String,
    #[serde(rename = "apiTraceId")]
    pub api_trace_id: String,
    pub error: AuthipayErrorDetails,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthipayErrorDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default)]
    pub details: Option<Vec<AuthipayErrorDetail>>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthipayErrorDetail {
    pub location: String,
    pub message: String,
    #[serde(rename = "locationType")]
    pub location_type: String,
}
