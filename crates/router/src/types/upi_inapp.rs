//! Types for UPI InApp payment method
//!
//! This module contains types for handling UPI InApp payments including
//! SDK parameter generation, PSP integration, and transaction mode detection.

use common_enums::enums;
use common_utils::{
    crypto::{HmacSha256, SignMessage},
    errors::{CustomResult, CryptoError},
    pii,
};
use error_stack::{ResultExt, report};
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

/// Custom error type for UPI InApp operations
#[derive(Debug, thiserror::Error)]
pub enum UpiInAppError {
    #[error("Invalid PSP: {0}")]
    InvalidPsp(String),
    #[error("Invalid auth type for UPI InApp")]
    InvalidAuthType,
    #[error("Signature generation failed: {0}")]
    SignatureGenerationFailed(String),
    #[error("Missing required field: {0}")]
    MissingField(&'static str),
}

/// Payment Service Provider (PSP) types for UPI InApp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UPIPSP {
    /// Axis Biz PSP - uses HMAC-SHA256 signature
    AxisBiz,
    /// Yes Biz PSP - uses RSA signature
    YesBiz,
    /// BHIM PSP - uses RSA signature
    Bhim,
}

impl UPIPSP {
    /// Parse PSP from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "AXIS_BIZ" | "AXISBIZ" => Some(Self::AxisBiz),
            "YES_BIZ" | "YESBIZ" => Some(Self::YesBiz),
            "BHIM" => Some(Self::Bhim),
            _ => None,
        }
    }

    /// Get PSP name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AxisBiz => "AXIS_BIZ",
            Self::YesBiz => "YES_BIZ",
            Self::Bhim => "BHIM",
        }
    }
}

/// Payment mode detected from gateway response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UPIPaymentMode {
    /// Standard bank account payment via UPI
    Standard,
    /// Credit card payment via UPI
    CreditCard,
    /// Prepaid instrument (PPI) payment via UPI
    PrepaidInstrument,
    /// Credit line payment via UPI
    CreditLine,
}

impl Default for UPIPaymentMode {
    fn default() -> Self {
        Self::Standard
    }
}

/// Payment mode detected from gateway response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UPIInAppPaymentMode {
    /// Standard bank account payment via UPI InApp
    Standard,
    /// Credit card payment via UPI InApp
    CreditCard,
    /// Prepaid instrument (PPI) payment via UPI InApp
    PrepaidInstrument,
    /// Credit line payment via UPI InApp
    CreditLine,
}

impl Default for UPIInAppPaymentMode {
    fn default() -> Self {
        Self::Standard
    }
}

/// Split settlement details for marketplace transactions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpiInAppSplitSettlementDetails {
    /// Split type (e.g., "AMOUNT")
    pub split_type: String,
    /// Marketplace/merchant split amount
    pub merchant_split: String,
    /// Partner splits with partner ID and value
    pub partners_split: Vec<PartnerSplit>,
}

/// Partner split detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartnerSplit {
    /// Partner ID (typically gateway sub-mid)
    pub partner_id: String,
    /// Split value/amount
    pub value: String,
}

/// PSP account details for UPI InApp
#[derive(Debug, Clone)]
pub struct UpiInAppPSPAccountDetails {
    /// PSP type
    pub psp: UPIPSP,
    /// Merchant ID for the PSP
    pub merchant_id: Secret<String>,
    /// Channel ID for the PSP
    pub channel_id: Secret<String>,
    /// MCC (Merchant Category Code)
    pub mcc: Secret<String>,
    /// PSP-specific prefix
    pub prefix: Secret<String>,
    /// Signing key (HMAC key for AxisBiz, private key for YesBiz/BHIM)
    pub signing_key: Secret<String>,
    /// Remarks (optional)
    pub remarks: Option<Secret<String>>,
    /// Key ID for JWS (optional)
    pub kid: Option<Secret<String>>,
    /// Algorithm for JWS (optional)
    pub alg: Option<String>,
}

impl TryFrom<&ConnectorAuthType> for UpiInAppPSPAccountDetails {
    type Error = UpiInAppError;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey { api_key, key1, api_secret } => {
                // Default to AxisBiz for SignatureKey auth
                Ok(Self {
                    psp: UPIPSP::AxisBiz,
                    merchant_id: api_key.clone(),
                    channel_id: key1.clone(),
                    mcc: key1.clone(),
                    prefix: key1.clone(),
                    signing_key: api_secret.clone(),
                    remarks: None,
                    kid: None,
                    alg: None,
                })
            }
            ConnectorAuthType::CertificateAuth { certificate: _, private_key } => {
                // Use for YesBiz/BHIM with RSA private key
                Ok(Self {
                    psp: UPIPSP::YesBiz,
                    merchant_id: private_key.clone(),
                    channel_id: private_key.clone(),
                    mcc: private_key.clone(),
                    prefix: private_key.clone(),
                    signing_key: private_key.clone(),
                    remarks: None,
                    kid: None,
                    alg: None,
                })
            }
            _ => Err(UpiInAppError::InvalidAuthType),
        }
    }
}

/// Request for generating UPI InApp SDK parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiInAppSDKParamsRequest {
    /// Merchant ID
    pub merchant_id: String,
    /// Customer ID
    pub customer_id: Option<String>,
    /// Customer mobile number
    pub mobile_number: Option<Secret<String>>,
    /// Payer VPA (customer's VPA)
    pub payer_vpa: Secret<String, pii::UpiVpaMaskingStrategy>,
    /// Payee VPA (merchant's VPA)
    pub payee_vpa: Secret<String, pii::UpiVpaMaskingStrategy>,
    /// Transaction amount
    pub amount: String,
    /// Currency code (default: INR)
    pub currency: Option<String>,
    /// Transaction reference
    pub transaction_reference_id: String,
    /// Bank account reference ID
    pub bank_account_reference_id: String,
    /// PSP to use
    pub psp: Option<String>,
    /// Issuing gateway
    pub issuing_gateway: Option<String>,
    /// Payment source/app name
    pub upi_app: Option<String>,
    /// Purpose of the transaction
    pub purpose: Option<String>,
    /// Split settlement details
    pub split_settlement: Option<UpiInAppSplitSettlementDetails>,
    /// Command type (SESSION_PARAMS or ENCRYPT_PAYLOAD)
    pub command: Option<String>,
}

/// Response for UPI InApp SDK parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiInAppSDKParamsResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// SDK parameters (session params or encrypted payload)
    pub sdk_params: Option<UpiInAppTransactionSDKParams>,
}

/// SDK parameters for UPI InApp transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiInAppTransactionSDKParams {
    /// Merchant request ID
    pub merchant_request_id: String,
    /// Customer VPA
    pub customer_vpa: String,
    /// Merchant VPA
    pub merchant_vpa: String,
    /// Transaction amount
    pub amount: String,
    /// Currency
    pub currency: String,
    /// Transaction reference
    pub transaction_reference_id: String,
    /// Cryptographic signature
    pub signature: String,
    /// Timestamp of generation
    pub timestamp: String,
    /// PSP used
    pub psp: String,
    /// Payment source/app
    pub upi_app: Option<String>,
    /// Split settlement details (JSON encoded)
    pub split_settlement: Option<String>,
    /// Purpose
    pub purpose: Option<String>,
    /// UPI ID generated by PSP
    pub upi_id: Option<String>,
}

/// Request for creating UPI InApp mandate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiInAppMandateSDKParamsRequest {
    /// Merchant ID
    pub merchant_id: String,
    /// Customer ID
    pub customer_id: String,
    /// Transaction reference
    pub transaction_reference_id: String,
    /// Payer VPA (customer's VPA)
    pub payer_vpa: Secret<String, pii::UpiVpaMaskingStrategy>,
    /// Payee VPA (merchant's VPA)
    pub payee_vpa: Secret<String, pii::UpiVpaMaskingStrategy>,
    /// Recipient name
    pub recipient_name: Option<String>,
    /// Mandate amount
    pub amount: String,
    /// Amount rule (FIXED or MAX)
    pub amount_rule: String,
    /// Recurrence pattern (DAILY, WEEKLY, MONTHLY, YEARLY, ONDemand)
    pub recurrence_pattern: String,
    /// Recurrence rule
    pub recurrence_rule: String,
    /// Recurrence value
    pub recurrence_value: Option<i32>,
    /// Mandate validity start date
    pub validity_start: String,
    /// Mandate validity end date
    pub validity_end: String,
    /// Purpose of the mandate
    pub purpose: Option<String>,
    /// PSP to use
    pub psp: Option<String>,
    /// Bank account reference ID
    pub bank_account_reference_id: Option<String>,
    /// Block fund indicator
    pub block_fund: Option<bool>,
}

/// Response for UPI InApp mandate SDK parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiInAppMandateSDKParamsResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Mandate SDK parameters
    pub mandate_params: Option<UpiInAppCreateMandateSDKParams>,
}

/// SDK parameters for UPI InApp mandate creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiInAppCreateMandateSDKParams {
    /// Merchant request ID
    pub merchant_request_id: String,
    /// Customer VPA
    pub customer_vpa: String,
    /// Merchant VPA
    pub merchant_vpa: String,
    /// Mandate amount
    pub amount: String,
    /// Currency
    pub currency: String,
    /// Transaction reference
    pub transaction_reference_id: String,
    /// Cryptographic signature
    pub signature: String,
    /// Timestamp of generation
    pub timestamp: String,
    /// PSP used
    pub psp: String,
    /// Recipient name
    pub recipient_name: String,
    /// Amount rule
    pub amount_rule: String,
    /// Recurrence pattern
    pub recurrence_pattern: String,
    /// Recurrence rule
    pub recurrence_rule: String,
    /// Recurrence value
    pub recurrence_value: Option<i32>,
    /// Mandate validity start date
    pub validity_start: String,
    /// Mandate validity end date
    pub validity_end: String,
    /// Purpose
    pub purpose: Option<String>,
    /// Block fund indicator
    pub block_fund: Option<bool>,
}

/// Session parameters for UPI InApp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiInAppSessionParams {
    /// Merchant ID
    pub merchant_id: String,
    /// Channel ID
    pub channel_id: String,
    /// Customer ID
    pub customer_id: String,
    /// MCC (Merchant Category Code)
    pub mcc: String,
    /// Timestamp
    pub timestamp: String,
    /// Currency
    pub currency: String,
    /// Cryptographic signature
    pub signature: String,
    /// PSP-specific prefix
    pub prefix: String,
    /// UDF parameters
    pub udf: Option<String>,
    /// VPA with reference ID and gateway
    pub vpa_with_ref_id_and_gw: Vec<VpaWithGwRefIdAndGw>,
    /// Mobile number (optional)
    pub mobile_number: Option<String>,
}

/// VPA with gateway reference ID and gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpaWithGwRefIdAndGw {
    /// VPA
    pub vpa: String,
    /// Reference ID
    pub ref_id: String,
    /// Gateway
    pub gateway: String,
}

/// PSP configuration for UPI InApp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiInAppPSPConfig {
    /// PSP identifier
    pub psp: UPIPSP,
    /// Whether JWS encryption is enabled
    pub jws_enabled: bool,
    /// JWS key ID
    pub kid: Option<String>,
    /// JWS algorithm
    pub alg: Option<String>,
    /// MGA (Merchant Gateway Account) entries
    pub mga: Vec<MGAEntry>,
}

/// MGA (Merchant Gateway Account) entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MGAEntry {
    /// Gateway
    pub gateway: String,
    /// Reference ID
    pub ref_id: String,
    /// VPA
    pub vpa: String,
}

/// Payment source structure for UPI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiPaymentSource {
    /// UPI identifier (e.g., "UPI_PAY", "UPI_INAPP")
    pub upi_identifier: String,
    /// UPI app name (optional)
    pub upi_app: Option<String>,
    /// Payer VPA (optional)
    pub payer_vpa: Option<String>,
}

impl UpiPaymentSource {
    /// Serialize to JSON string
    pub fn to_json_string(&self) -> error_stack::Result<String, serde_json::Error> {
        serde_json::to_string(self).map_err(|e| error_stack::Report::new(e))
    }

    /// Deserialize from JSON string
    pub fn from_json_string(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }
}

/// Utility functions for UPI InApp signature generation
pub mod signature_utils {
    use super::*;

    /// Generate HMAC-SHA256 signature for AxisBiz PSP
    pub fn generate_hmac_signature(
        payload: &str,
        signing_key: &Secret<String>,
    ) -> CustomResult<String, CryptoError> {
        let key = signing_key.peek().as_bytes();
        let msg = payload.as_bytes();

        let hmac = HmacSha256;
        let signature = hmac.sign_message(key, msg)?;

        Ok(base64::encode(signature))
    }

    /// Format split settlement details for signature payload
    pub fn format_split_details(
        details: &Option<UpiInAppSplitSettlementDetails>,
    ) -> String {
        match details {
            Some(split) => {
                let partner_splits: String = split
                    .partners_split
                    .iter()
                    .map(|p| format!("{}{}", p.partner_id, p.value))
                    .collect();

                format!(
                    "{}{}{}",
                    split.merchant_split, partner_splits, split.split_type
                )
            }
            None => String::new(),
        }
    }

    /// Build signature payload in alphabetical order
    pub fn build_signature_payload(
        currency: &str,
        mobile_number: &Option<Secret<String>>,
        mcc: &str,
        channel_id: &str,
        customer_id: &str,
        merchant_id: &str,
        timestamp: &str,
        account_reference_id: &str,
        amount: &str,
        payer_vpa: &str,
        payee_vpa: &str,
        transaction_reference: &str,
        split_details: &str,
    ) -> String {
        let mobile = mobile_number
            .as_ref()
            .map(|m| m.peek().clone())
            .unwrap_or_default();

        // Format: currency + mobile + mcc + channelId + customerId + merchantId + timestamp + accountRef + amount + payerVPA + payeeVPA + transactionRef + splitDetails
        format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}{}",
            currency,
            mobile,
            mcc,
            channel_id,
            customer_id,
            merchant_id,
            timestamp,
            account_reference_id,
            amount,
            payer_vpa,
            payee_vpa,
            transaction_reference,
            split_details
        )
    }
}
