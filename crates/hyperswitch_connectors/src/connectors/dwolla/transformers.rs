use common_enums::enums;
use common_utils::{
    types::{StringMajorUnit, MinorUnit},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        refunds::{Execute, RSync},
    },
    router_request_types::{AccessTokenRequestData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, AddressDetailsData, RouterData as RouterDataTrait},
};

// Router Data wrapper for amount conversion
pub struct DwollaRouterData<T> {
    pub amount: StringMajorUnit, // Dwolla accepts amounts in major units (dollars)
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for DwollaRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// Auth Structures
#[derive(Debug, Clone)]
pub struct DwollaAuthType {
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for DwollaAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                client_id: api_key.to_owned(),
                client_secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// OAuth Token Request/Response
#[derive(Debug, Serialize)]
pub struct DwollaTokenRequest {
    pub grant_type: String,
}

impl TryFrom<&RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>> for DwollaTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(_item: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>) -> Result<Self, Self::Error> {
        Ok(Self {
            grant_type: "client_credentials".to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DwollaTokenResponse {
    pub access_token: Secret<String>,
    pub token_type: String,
    pub expires_in: i64,
}

impl TryFrom<ResponseRouterData<AccessTokenAuth, DwollaTokenResponse, AccessTokenRequestData, AccessToken>>
    for RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(
        item: ResponseRouterData<AccessTokenAuth, DwollaTokenResponse, AccessTokenRequestData, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

// Customer Creation Request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DwollaCustomerRequest {
    pub first_name: Secret<String>,
    pub last_name: Secret<String>,
    pub email: common_utils::pii::Email,
    #[serde(rename = "type")]
    pub customer_type: String,
    pub address1: Secret<String>,
    pub city: Secret<String>,
    pub state: Secret<String>,
    pub postal_code: Secret<String>,
    pub date_of_birth: String,
    pub ssn: Secret<String>,
}

// Funding Source (Bank Account) Request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DwollaFundingSourceRequest {
    pub routing_number: Secret<String>,
    pub account_number: Secret<String>,
    pub bank_account_type: String,
    pub name: String,
}

// Bank Account Verification Structures
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DwollaVerificationRequest {
    pub amount1: DwollaAmount,
    pub amount2: DwollaAmount,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DwollaVerificationResponse {
    #[serde(rename = "_links")]
    pub links: Option<DwollaResponseLinks>,
    pub id: String,
    pub status: DwollaVerificationStatus,
    pub created: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DwollaVerificationStatus {
    #[serde(rename = "verified")]
    Verified,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "unverified")]
    Unverified,
}

// Micro-deposit initiation request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DwollaMicroDepositRequest {
    #[serde(rename = "_links")]
    pub links: DwollaMicroDepositLinks,
}

#[derive(Debug, Serialize)]
pub struct DwollaMicroDepositLinks {
    #[serde(rename = "funding-source")]
    pub funding_source: DwollaLink,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DwollaMicroDepositResponse {
    #[serde(rename = "_links")]
    pub links: Option<DwollaResponseLinks>,
    pub id: String,
    pub status: String,
    pub created: Option<String>,
    pub failure_reason: Option<String>,
}

// Funding Source Response with verification status
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DwollaFundingSourceResponse {
    #[serde(rename = "_links")]
    pub links: Option<DwollaResponseLinks>,
    pub id: String,
    pub status: String,
    pub bank_account_type: Option<String>,
    pub name: String,
    pub created: Option<String>,
    pub removed: Option<bool>,
    pub channels: Option<Vec<String>>,
    pub bank_name: Option<String>,
    pub fingerprint: Option<String>,
}

// Transfer Request (for payments and refunds)
#[derive(Debug, Serialize)]
pub struct DwollaTransferRequest {
    #[serde(rename = "_links")]
    pub links: DwollaTransferLinks,
    pub amount: DwollaAmount,
    pub metadata: Option<DwollaMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clearing: Option<DwollaClearingOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "correlationId")]
    pub correlation_id: Option<String>,
}

// Same Day ACH Support Structures
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DwollaClearingOptions {
    pub destination: DwollaClearingDestination,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<DwollaClearingSource>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DwollaClearingDestination {
    #[serde(rename = "next-available")]
    NextAvailable,
    #[serde(rename = "same-day")]
    SameDay,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DwollaClearingSource {
    #[serde(rename = "standard")]
    Standard,
    #[serde(rename = "same-day")]
    SameDay,
}

// Same Day ACH Configuration
#[derive(Debug, Clone)]
pub struct SameDayAchConfig {
    pub enabled: bool,
    pub cutoff_time_utc: String, // Format: "HH:MM" in UTC
    pub max_amount_cents: i64,   // Maximum amount for same day ACH in cents
    pub business_days_only: bool,
}

impl Default for SameDayAchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cutoff_time_utc: "15:45".to_string(), // 3:45 PM UTC (typical same-day ACH cutoff)
            max_amount_cents: 100_000_00,         // $100,000 default limit
            business_days_only: true,
        }
    }
}

impl SameDayAchConfig {
    pub fn new(enabled: bool, cutoff_time_utc: String, max_amount_cents: i64) -> Self {
        Self {
            enabled,
            cutoff_time_utc,
            max_amount_cents,
            business_days_only: true,
        }
    }

    pub fn is_same_day_eligible(&self, amount_cents: i64) -> bool {
        self.enabled && amount_cents <= self.max_amount_cents && self.is_within_cutoff_time()
    }

    pub fn is_within_cutoff_time(&self) -> bool {
        if !self.enabled {
            return false;
        }

        let now = common_utils::date_time::now();
        
        // Check if it's a business day (Monday-Friday)
        if self.business_days_only {
            let weekday = now.weekday();
            if weekday == time::Weekday::Saturday || weekday == time::Weekday::Sunday {
                return false;
            }
        }

        // Parse cutoff time
        if let Some((hour, minute)) = self.parse_cutoff_time() {
            let current_hour = now.hour();
            let current_minute = now.minute();
            
            // Check if current time is before cutoff
            if current_hour < hour || (current_hour == hour && current_minute <= minute) {
                return true;
            }
        }

        false
    }

    fn parse_cutoff_time(&self) -> Option<(u8, u8)> {
        let parts: Vec<&str> = self.cutoff_time_utc.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(hour), Ok(minute)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>()) {
                if hour < 24 && minute < 60 {
                    return Some((hour, minute));
                }
            }
        }
        None
    }

    pub fn get_clearing_option(&self, amount_cents: i64) -> DwollaClearingDestination {
        if self.is_same_day_eligible(amount_cents) {
            DwollaClearingDestination::SameDay
        } else {
            DwollaClearingDestination::NextAvailable
        }
    }
}

#[derive(Debug, Serialize, Default)]
pub struct DwollaTransferLinks {
    pub source: DwollaLink,
    pub destination: DwollaLink,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct DwollaLink {
    pub href: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct DwollaAmount {
    pub currency: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DwollaMetadata {
    pub order_id: String,
    pub customer_reference: String,
}

// Enhanced metadata for multi-step ACH flows
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DwollaPaymentMetadata {
    pub customer_id: Option<String>,
    pub funding_source_id: Option<String>,
    pub transfer_id: Option<String>,
    pub verification_status: Option<String>,
    pub micro_deposit_id: Option<String>,
    pub step_completed: DwollaPaymentStep,
    pub created_at: String,
    pub last_updated: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DwollaPaymentStep {
    CustomerCreation,
    FundingSourceCreation,
    VerificationInitiated,
    VerificationCompleted,
    TransferCreated,
    TransferCompleted,
    Failed,
}

impl Default for DwollaPaymentStep {
    fn default() -> Self {
        Self::CustomerCreation
    }
}

impl DwollaPaymentMetadata {
    pub fn new() -> Self {
        let now = common_utils::date_time::now().to_string();
        Self {
            customer_id: None,
            funding_source_id: None,
            transfer_id: None,
            verification_status: None,
            micro_deposit_id: None,
            step_completed: DwollaPaymentStep::CustomerCreation,
            created_at: now.clone(),
            last_updated: now,
        }
    }

    pub fn update_step(&mut self, step: DwollaPaymentStep) {
        self.step_completed = step;
        self.last_updated = common_utils::date_time::now().to_string();
    }

    pub fn set_customer_id(&mut self, customer_id: String) {
        self.customer_id = Some(customer_id);
        self.update_step(DwollaPaymentStep::CustomerCreation);
    }

    pub fn set_funding_source_id(&mut self, funding_source_id: String) {
        self.funding_source_id = Some(funding_source_id);
        self.update_step(DwollaPaymentStep::FundingSourceCreation);
    }

    pub fn set_verification_initiated(&mut self, micro_deposit_id: String) {
        self.micro_deposit_id = Some(micro_deposit_id);
        self.verification_status = Some("pending".to_string());
        self.update_step(DwollaPaymentStep::VerificationInitiated);
    }

    pub fn set_verification_completed(&mut self) {
        self.verification_status = Some("verified".to_string());
        self.update_step(DwollaPaymentStep::VerificationCompleted);
    }

    pub fn set_transfer_id(&mut self, transfer_id: String) {
        self.transfer_id = Some(transfer_id);
        self.update_step(DwollaPaymentStep::TransferCreated);
    }

    pub fn set_transfer_completed(&mut self) {
        self.update_step(DwollaPaymentStep::TransferCompleted);
    }

    pub fn set_failed(&mut self) {
        self.update_step(DwollaPaymentStep::Failed);
    }

    pub fn is_customer_created(&self) -> bool {
        self.customer_id.is_some()
    }

    pub fn is_funding_source_created(&self) -> bool {
        self.funding_source_id.is_some()
    }

    pub fn is_verification_required(&self) -> bool {
        matches!(self.step_completed, DwollaPaymentStep::FundingSourceCreation)
    }

    pub fn is_verification_completed(&self) -> bool {
        matches!(self.step_completed, DwollaPaymentStep::VerificationCompleted)
    }

    pub fn is_ready_for_transfer(&self) -> bool {
        self.is_verification_completed() || 
        (self.is_funding_source_created() && self.verification_status.as_deref() == Some("verified"))
    }

    pub fn get_next_step(&self) -> DwollaPaymentStep {
        match self.step_completed {
            DwollaPaymentStep::CustomerCreation => {
                if self.is_customer_created() {
                    DwollaPaymentStep::FundingSourceCreation
                } else {
                    DwollaPaymentStep::CustomerCreation
                }
            }
            DwollaPaymentStep::FundingSourceCreation => {
                if self.is_funding_source_created() {
                    DwollaPaymentStep::VerificationInitiated
                } else {
                    DwollaPaymentStep::FundingSourceCreation
                }
            }
            DwollaPaymentStep::VerificationInitiated => DwollaPaymentStep::VerificationCompleted,
            DwollaPaymentStep::VerificationCompleted => DwollaPaymentStep::TransferCreated,
            DwollaPaymentStep::TransferCreated => DwollaPaymentStep::TransferCompleted,
            DwollaPaymentStep::TransferCompleted => DwollaPaymentStep::TransferCompleted,
            DwollaPaymentStep::Failed => DwollaPaymentStep::Failed,
        }
    }

    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json_string(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for DwollaPaymentMetadata {
    fn default() -> Self {
        Self::new()
    }
}

// Payment Request (combines multiple steps)
#[derive(Debug, Serialize)]
pub struct DwollaPaymentsRequest {
    pub customer: DwollaCustomerRequest,
    pub funding_source: DwollaFundingSourceRequest,
    pub transfer: DwollaTransferRequest,
}

impl TryFrom<&DwollaRouterData<&PaymentsAuthorizeRouterData>> for DwollaPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: &DwollaRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        let billing_address = item.router_data.get_billing_address()?;
        
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankDebit(bank_debit) => {
                let customer = DwollaCustomerRequest {
                    first_name: billing_address.get_first_name()?.clone(),
                    last_name: billing_address.get_last_name()?.clone(),
                    email: item.router_data.request.email.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "email",
                        }
                    )?,
                    customer_type: "personal".to_string(),
                    address1: billing_address.get_line1()?.clone(),
                    city: Secret::new(billing_address.get_city()?.clone()),
                    state: billing_address.get_state()?.clone(),
                    postal_code: billing_address.get_zip()?.clone(),
                    date_of_birth: "1990-01-01".to_string(), // This should come from customer data
                    ssn: Secret::new("1234".to_string()), // Last 4 digits of SSN
                };

                // Extract bank account details from BankDebitData
                let (routing_number, account_number) = match &bank_debit {
                    hyperswitch_domain_models::payment_method_data::BankDebitData::AchBankDebit { 
                        routing_number, 
                        account_number, 
                        .. 
                    } => (routing_number.clone(), account_number.clone()),
                    _ => return Err(errors::ConnectorError::NotImplemented("Bank debit type".to_string()).into()),
                };

                let funding_source = DwollaFundingSourceRequest {
                    routing_number,
                    account_number,
                    bank_account_type: "checking".to_string(), // Default to checking for now
                    name: "Bank Account".to_string(), // Default name since account_holder_name is not available
                };

                let transfer = DwollaTransferRequest {
                    links: DwollaTransferLinks {
                        source: DwollaLink {
                            href: "https://api.dwolla.com/funding-sources/customer-account-id".to_string(),
                        },
                        destination: DwollaLink {
                            href: "https://api.dwolla.com/funding-sources/merchant-account-id".to_string(),
                        },
                    },
                    amount: DwollaAmount {
                        currency: item.router_data.request.currency.to_string(),
                        value: item.amount.get_amount_as_string(),
                    },
                    metadata: Some(DwollaMetadata {
                        order_id: item.router_data.connector_request_reference_id.clone(),
                        customer_reference: item.router_data.payment_id.clone(),
                    }),
                    clearing: None, // Will be set based on same-day ACH eligibility
                    correlation_id: None, // Optional correlation ID for tracking
                };

                Ok(Self {
                    customer,
                    funding_source,
                    transfer,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Payment Response
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DwollaPaymentStatus {
    #[serde(rename = "processed")]
    Processed,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "reclaimed")]
    Reclaimed,
    #[default]
    Processing,
}

impl From<DwollaPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: DwollaPaymentStatus) -> Self {
        match item {
            DwollaPaymentStatus::Processed => Self::Charged,
            DwollaPaymentStatus::Pending => Self::Pending,
            DwollaPaymentStatus::Failed => Self::Failure,
            DwollaPaymentStatus::Cancelled => Self::Voided,
            DwollaPaymentStatus::Reclaimed => Self::Failure,
            DwollaPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DwollaPaymentsResponse {
    #[serde(rename = "_links")]
    pub links: Option<DwollaResponseLinks>,
    pub id: String,
    pub status: DwollaPaymentStatus,
    pub amount: Option<DwollaAmount>,
    pub created: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DwollaResponseLinks {
    #[serde(rename = "self")]
    pub self_link: Option<DwollaLink>,
    pub source: Option<DwollaLink>,
    pub destination: Option<DwollaLink>,
}

impl<F, T> TryFrom<ResponseRouterData<F, DwollaPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(
        item: ResponseRouterData<F, DwollaPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response_id = item.response.id.clone();
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(response_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// Refund Request
#[derive(Default, Debug, Serialize)]
pub struct DwollaRefundRequest {
    #[serde(rename = "_links")]
    pub links: DwollaTransferLinks,
    pub amount: DwollaAmount,
}

impl<F> TryFrom<&DwollaRouterData<&RefundsRouterData<F>>> for DwollaRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: &DwollaRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            links: DwollaTransferLinks {
                source: DwollaLink {
                    href: "https://api.dwolla.com/funding-sources/merchant-account-id".to_string(),
                },
                destination: DwollaLink {
                    href: "https://api.dwolla.com/funding-sources/customer-account-id".to_string(),
                },
            },
            amount: DwollaAmount {
                currency: item.router_data.request.currency.to_string(),
                value: item.amount.get_amount_as_string(),
            },
        })
    }
}

// Refund Response
#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum RefundStatus {
    #[serde(rename = "processed")]
    Processed,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Processed => Self::Success,
            RefundStatus::Pending => Self::Pending,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Cancelled => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    #[serde(rename = "_links")]
    pub links: Option<DwollaResponseLinks>,
    pub id: String,
    pub status: RefundStatus,
    pub amount: Option<DwollaAmount>,
    pub created: Option<String>,
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
    
    fn try_from(item: RefundsResponseRouterData<RSync, RefundResponse>) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

// Error Response
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaErrorResponse {
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    #[serde(rename = "_embedded")]
    pub embedded: Option<DwollaEmbeddedErrors>,
}

// OAuth Token Error Response (different format)
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaOAuthErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}

impl From<DwollaOAuthErrorResponse> for DwollaErrorResponse {
    fn from(oauth_error: DwollaOAuthErrorResponse) -> Self {
        Self {
            code: oauth_error.error,
            message: oauth_error.error_description.unwrap_or_else(|| "OAuth error".to_string()),
            reason: None,
            embedded: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaEmbeddedErrors {
    pub errors: Vec<DwollaError>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaError {
    pub code: String,
    pub message: String,
    pub path: Option<String>,
}

// ACH Return Code Structures
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaTransferFailure {
    #[serde(rename = "_links")]
    pub links: Option<DwollaResponseLinks>,
    pub code: String,
    pub description: String,
    pub explanation: Option<String>,
}

// ACH Return Code Mappings
#[derive(Debug, Clone, PartialEq)]
pub enum AchReturnCode {
    R01, // Insufficient Funds
    R02, // Account Closed
    R03, // No Account/Unable to Locate Account
    R04, // Invalid Account Number Structure
    R05, // Unauthorized Debit to Consumer Account Using Corporate SEC Code
    R06, // Returned per ODFI's Request
    R07, // Authorization Revoked by Customer
    R08, // Payment Stopped
    R09, // Uncollected Funds
    R10, // Customer Advises Not Authorized
    R11, // Check Truncation Entry Return
    R12, // Branch Sold to Another DFI
    R13, // Invalid ACH Routing Number
    R14, // Representative Payee Deceased or Unable to Continue
    R15, // Beneficiary or Account Holder Deceased
    R16, // Account Frozen
    R17, // File Record Edit Criteria
    R18, // Improper Effective Entry Date
    R19, // Amount Field Error
    R20, // Non-Transaction Account
    R21, // Invalid Company Identification
    R22, // Invalid Individual ID Number
    R23, // Credit Entry Refused by Receiver
    R24, // Duplicate Entry
    R25, // Addenda Error
    R26, // Mandatory Field Error
    R27, // Trace Number Error
    R28, // Routing Number Check Digit Error
    R29, // Corporate Customer Advises Not Authorized
    R30, // RDFI Not Participant in Check Truncation Program
    R31, // Permissible Return Entry
    R32, // RDFI Non-Settlement
    R33, // Return of XCK Entry
    Unknown(String), // For any other return codes
}

impl From<&str> for AchReturnCode {
    fn from(code: &str) -> Self {
        match code {
            "R01" => AchReturnCode::R01,
            "R02" => AchReturnCode::R02,
            "R03" => AchReturnCode::R03,
            "R04" => AchReturnCode::R04,
            "R05" => AchReturnCode::R05,
            "R06" => AchReturnCode::R06,
            "R07" => AchReturnCode::R07,
            "R08" => AchReturnCode::R08,
            "R09" => AchReturnCode::R09,
            "R10" => AchReturnCode::R10,
            "R11" => AchReturnCode::R11,
            "R12" => AchReturnCode::R12,
            "R13" => AchReturnCode::R13,
            "R14" => AchReturnCode::R14,
            "R15" => AchReturnCode::R15,
            "R16" => AchReturnCode::R16,
            "R17" => AchReturnCode::R17,
            "R18" => AchReturnCode::R18,
            "R19" => AchReturnCode::R19,
            "R20" => AchReturnCode::R20,
            "R21" => AchReturnCode::R21,
            "R22" => AchReturnCode::R22,
            "R23" => AchReturnCode::R23,
            "R24" => AchReturnCode::R24,
            "R25" => AchReturnCode::R25,
            "R26" => AchReturnCode::R26,
            "R27" => AchReturnCode::R27,
            "R28" => AchReturnCode::R28,
            "R29" => AchReturnCode::R29,
            "R30" => AchReturnCode::R30,
            "R31" => AchReturnCode::R31,
            "R32" => AchReturnCode::R32,
            "R33" => AchReturnCode::R33,
            other => AchReturnCode::Unknown(other.to_string()),
        }
    }
}

impl AchReturnCode {
    pub fn get_error_reason(&self) -> &'static str {
        match self {
            AchReturnCode::R01 => "Insufficient Funds",
            AchReturnCode::R02 => "Account Closed",
            AchReturnCode::R03 => "No Account/Unable to Locate Account",
            AchReturnCode::R04 => "Invalid Account Number Structure",
            AchReturnCode::R05 => "Unauthorized Debit to Consumer Account Using Corporate SEC Code",
            AchReturnCode::R06 => "Returned per ODFI's Request",
            AchReturnCode::R07 => "Authorization Revoked by Customer",
            AchReturnCode::R08 => "Payment Stopped",
            AchReturnCode::R09 => "Uncollected Funds",
            AchReturnCode::R10 => "Customer Advises Not Authorized",
            AchReturnCode::R11 => "Check Truncation Entry Return",
            AchReturnCode::R12 => "Branch Sold to Another DFI",
            AchReturnCode::R13 => "Invalid ACH Routing Number",
            AchReturnCode::R14 => "Representative Payee Deceased or Unable to Continue",
            AchReturnCode::R15 => "Beneficiary or Account Holder Deceased",
            AchReturnCode::R16 => "Account Frozen",
            AchReturnCode::R17 => "File Record Edit Criteria",
            AchReturnCode::R18 => "Improper Effective Entry Date",
            AchReturnCode::R19 => "Amount Field Error",
            AchReturnCode::R20 => "Non-Transaction Account",
            AchReturnCode::R21 => "Invalid Company Identification",
            AchReturnCode::R22 => "Invalid Individual ID Number",
            AchReturnCode::R23 => "Credit Entry Refused by Receiver",
            AchReturnCode::R24 => "Duplicate Entry",
            AchReturnCode::R25 => "Addenda Error",
            AchReturnCode::R26 => "Mandatory Field Error",
            AchReturnCode::R27 => "Trace Number Error",
            AchReturnCode::R28 => "Routing Number Check Digit Error",
            AchReturnCode::R29 => "Corporate Customer Advises Not Authorized",
            AchReturnCode::R30 => "RDFI Not Participant in Check Truncation Program",
            AchReturnCode::R31 => "Permissible Return Entry",
            AchReturnCode::R32 => "RDFI Non-Settlement",
            AchReturnCode::R33 => "Return of XCK Entry",
            AchReturnCode::Unknown(_code) => "Unknown ACH Return Code",
        }
    }

    pub fn to_hyperswitch_error(&self) -> errors::ConnectorError {
        match self {
            AchReturnCode::R01 => errors::ConnectorError::InSufficientBalanceInPaymentMethod,
            AchReturnCode::R02 | AchReturnCode::R03 | AchReturnCode::R04 | AchReturnCode::R16 => {
                errors::ConnectorError::InvalidDataFormat { field_name: "account_number" }
            }
            AchReturnCode::R05 | AchReturnCode::R07 | AchReturnCode::R10 | AchReturnCode::R29 => {
                errors::ConnectorError::FailedAtConnector { 
                    message: "Unauthorized transaction".to_string(),
                    code: "unauthorized".to_string()
                }
            }
            AchReturnCode::R08 => errors::ConnectorError::FailedAtConnector { 
                message: "Payment stopped by customer".to_string(),
                code: "payment_stopped".to_string()
            },
            AchReturnCode::R09 => errors::ConnectorError::InSufficientBalanceInPaymentMethod,
            AchReturnCode::R13 | AchReturnCode::R28 => errors::ConnectorError::InvalidDataFormat { field_name: "routing_number" },
            AchReturnCode::R15 => errors::ConnectorError::InvalidDataFormat { field_name: "account_number" },
            AchReturnCode::R19 => errors::ConnectorError::AmountConversionFailed,
            AchReturnCode::R20 => errors::ConnectorError::InvalidDataFormat { field_name: "account_type" },
            AchReturnCode::R21 | AchReturnCode::R22 => errors::ConnectorError::InvalidConnectorConfig { config: "merchant_details" },
            AchReturnCode::R23 => errors::ConnectorError::FailedAtConnector { 
                message: "Credit entry refused by receiver".to_string(),
                code: "refused_by_bank".to_string()
            },
            AchReturnCode::R24 => errors::ConnectorError::FailedAtConnector { 
                message: "Duplicate entry".to_string(),
                code: "duplicate_transaction".to_string()
            },
            AchReturnCode::R25 | AchReturnCode::R26 | AchReturnCode::R27 => {
                errors::ConnectorError::RequestEncodingFailedWithReason("Invalid request format".to_string())
            }
            AchReturnCode::R06 | AchReturnCode::R11 | AchReturnCode::R12 | AchReturnCode::R14 
            | AchReturnCode::R17 | AchReturnCode::R18 | AchReturnCode::R30 | AchReturnCode::R31 
            | AchReturnCode::R32 | AchReturnCode::R33 => {
                errors::ConnectorError::ProcessingStepFailed(None)
            }
            AchReturnCode::Unknown(_) => errors::ConnectorError::GenericError {
                error_message: "Unknown ACH return code".to_string(),
                error_object: serde_json::Value::Null,
            },
        }
    }
}

// Error mapping functions
pub fn map_dwolla_error_to_hyperswitch(
    error_response: &DwollaErrorResponse,
) -> errors::ConnectorError {
    // Check if this is an ACH return code error
    if let Some(ach_code) = extract_ach_return_code(&error_response.code) {
        return ach_code.to_hyperswitch_error();
    }

    // Handle standard Dwolla API errors
    match error_response.code.as_str() {
        "ValidationError" => errors::ConnectorError::RequestEncodingFailedWithReason("Validation error".to_string()),
        "Forbidden" => errors::ConnectorError::FailedAtConnector { 
            message: "Forbidden access".to_string(),
            code: "forbidden".to_string()
        },
        "NotFound" => errors::ConnectorError::FailedAtConnector { 
            message: "Resource not found".to_string(),
            code: "not_found".to_string()
        },
        "Unauthorized" => errors::ConnectorError::FailedToObtainAuthType,
        "InternalServerError" => errors::ConnectorError::ProcessingStepFailed(None),
        "BadGateway" => errors::ConnectorError::ProcessingStepFailed(None),
        "ServiceUnavailable" => errors::ConnectorError::ProcessingStepFailed(None),
        "GatewayTimeout" => errors::ConnectorError::RequestTimeoutReceived,
        _ => errors::ConnectorError::GenericError {
            error_message: "Unknown error".to_string(),
            error_object: serde_json::Value::Null,
        },
    }
}

pub fn extract_ach_return_code(error_code: &str) -> Option<AchReturnCode> {
    // Check if the error code matches ACH return code pattern (R01-R33)
    if error_code.len() == 3 && error_code.starts_with('R') {
        if let Ok(num) = error_code[1..].parse::<u8>() {
            if (1..=33).contains(&num) {
                return Some(AchReturnCode::from(error_code));
            }
        }
    }
    None
}

pub fn map_transfer_failure_to_hyperswitch(
    failure: &DwollaTransferFailure,
) -> errors::ConnectorError {
    // Check if this is an ACH return code
    if let Some(ach_code) = extract_ach_return_code(&failure.code) {
        return ach_code.to_hyperswitch_error();
    }

    // Handle other transfer failure codes
    match failure.code.as_str() {
        "InsufficientFunds" => errors::ConnectorError::InSufficientBalanceInPaymentMethod,
        "InvalidAccount" => errors::ConnectorError::InvalidDataFormat { field_name: "account_number" },
        "UnauthorizedTransaction" => errors::ConnectorError::FailedAtConnector { 
            message: "Unauthorized transaction".to_string(),
            code: "unauthorized".to_string()
        },
        "InvalidAmount" => errors::ConnectorError::AmountConversionFailed,
        "DuplicateResource" => errors::ConnectorError::FailedAtConnector { 
            message: "Duplicate resource".to_string(),
            code: "duplicate_transaction".to_string()
        },
        _ => errors::ConnectorError::ProcessingStepFailed(None),
    }
}

// Helper functions for amount conversion
pub fn to_currency_base_unit_asf64(
    amount: MinorUnit,
    currency: enums::Currency,
) -> Result<f64, error_stack::Report<errors::ConnectorError>> {
    let amount_decimal = utils::to_currency_base_unit(amount.get_amount_as_i64(), currency)?;
    amount_decimal
        .parse::<f64>()
        .change_context(errors::ConnectorError::AmountConversionFailed)
}

// Webhook Event Structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DwollaWebhookEvent {
    pub id: String,
    pub topic: String,
    #[serde(rename = "resourceId")]
    pub resource_id: String,
    pub timestamp: String,
    #[serde(rename = "_links")]
    pub links: DwollaWebhookLinks,
    #[serde(rename = "_embedded")]
    pub embedded: Option<DwollaWebhookEmbedded>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DwollaWebhookLinks {
    #[serde(rename = "self")]
    pub self_link: DwollaLink,
    pub resource: DwollaLink,
    pub account: Option<DwollaLink>,
    pub customer: Option<DwollaLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DwollaWebhookEmbedded {
    pub resource: Option<DwollaWebhookResource>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DwollaWebhookResource {
    pub id: String,
    pub status: Option<String>,
    pub amount: Option<DwollaAmount>,
    #[serde(rename = "created")]
    pub created_at: Option<String>,
    #[serde(rename = "_links")]
    pub links: Option<DwollaResponseLinks>,
}

// Enhanced webhook event types
#[derive(Debug, Clone, PartialEq)]
pub enum DwollaWebhookEventType {
    // Transfer events
    TransferCreated,
    TransferCompleted,
    TransferFailed,
    TransferCancelled,
    TransferReclaimed,
    
    // Customer transfer events
    CustomerTransferCreated,
    CustomerTransferCompleted,
    CustomerTransferFailed,
    CustomerTransferCancelled,
    
    // Bank transfer events
    BankTransferCreated,
    BankTransferCompleted,
    BankTransferFailed,
    BankTransferCancelled,
    
    // Mass payment events
    MassPaymentCreated,
    MassPaymentCompleted,
    MassPaymentCancelled,
    
    // Customer events
    CustomerCreated,
    CustomerVerificationDocumentNeeded,
    CustomerVerificationDocumentUploaded,
    CustomerVerificationDocumentFailed,
    CustomerVerificationDocumentApproved,
    CustomerReverificationNeeded,
    CustomerActivated,
    CustomerDeactivated,
    CustomerSuspended,
    
    // Funding source events
    FundingSourceAdded,
    FundingSourceRemoved,
    FundingSourceUnverified,
    FundingSourceNegative,
    
    // Webhook subscription events
    WebhookSubscriptionCreated,
    WebhookSubscriptionDeleted,
    WebhookSubscriptionPaused,
    
    // Unknown event type
    Unknown(String),
}

impl From<&str> for DwollaWebhookEventType {
    fn from(topic: &str) -> Self {
        match topic {
            // Transfer events
            "transfer_created" => DwollaWebhookEventType::TransferCreated,
            "transfer_completed" => DwollaWebhookEventType::TransferCompleted,
            "transfer_failed" => DwollaWebhookEventType::TransferFailed,
            "transfer_cancelled" => DwollaWebhookEventType::TransferCancelled,
            "transfer_reclaimed" => DwollaWebhookEventType::TransferReclaimed,
            
            // Customer transfer events
            "customer_transfer_created" => DwollaWebhookEventType::CustomerTransferCreated,
            "customer_transfer_completed" => DwollaWebhookEventType::CustomerTransferCompleted,
            "customer_transfer_failed" => DwollaWebhookEventType::CustomerTransferFailed,
            "customer_transfer_cancelled" => DwollaWebhookEventType::CustomerTransferCancelled,
            
            // Bank transfer events
            "bank_transfer_created" => DwollaWebhookEventType::BankTransferCreated,
            "bank_transfer_completed" => DwollaWebhookEventType::BankTransferCompleted,
            "bank_transfer_failed" => DwollaWebhookEventType::BankTransferFailed,
            "bank_transfer_cancelled" => DwollaWebhookEventType::BankTransferCancelled,
            
            // Mass payment events
            "mass_payment_created" => DwollaWebhookEventType::MassPaymentCreated,
            "mass_payment_completed" => DwollaWebhookEventType::MassPaymentCompleted,
            "mass_payment_cancelled" => DwollaWebhookEventType::MassPaymentCancelled,
            
            // Customer events
            "customer_created" => DwollaWebhookEventType::CustomerCreated,
            "customer_verification_document_needed" => DwollaWebhookEventType::CustomerVerificationDocumentNeeded,
            "customer_verification_document_uploaded" => DwollaWebhookEventType::CustomerVerificationDocumentUploaded,
            "customer_verification_document_failed" => DwollaWebhookEventType::CustomerVerificationDocumentFailed,
            "customer_verification_document_approved" => DwollaWebhookEventType::CustomerVerificationDocumentApproved,
            "customer_reverification_needed" => DwollaWebhookEventType::CustomerReverificationNeeded,
            "customer_activated" => DwollaWebhookEventType::CustomerActivated,
            "customer_deactivated" => DwollaWebhookEventType::CustomerDeactivated,
            "customer_suspended" => DwollaWebhookEventType::CustomerSuspended,
            
            // Funding source events
            "funding_source_added" => DwollaWebhookEventType::FundingSourceAdded,
            "funding_source_removed" => DwollaWebhookEventType::FundingSourceRemoved,
            "funding_source_unverified" => DwollaWebhookEventType::FundingSourceUnverified,
            "funding_source_negative" => DwollaWebhookEventType::FundingSourceNegative,
            
            // Webhook subscription events
            "webhook_subscription_created" => DwollaWebhookEventType::WebhookSubscriptionCreated,
            "webhook_subscription_deleted" => DwollaWebhookEventType::WebhookSubscriptionDeleted,
            "webhook_subscription_paused" => DwollaWebhookEventType::WebhookSubscriptionPaused,
            
            // Unknown event type
            other => DwollaWebhookEventType::Unknown(other.to_string()),
        }
    }
}

impl DwollaWebhookEventType {
    pub fn to_hyperswitch_event(&self) -> api_models::webhooks::IncomingWebhookEvent {
        match self {
            // Payment success events
            DwollaWebhookEventType::TransferCompleted 
            | DwollaWebhookEventType::CustomerTransferCompleted 
            | DwollaWebhookEventType::BankTransferCompleted => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
            }
            
            // Payment processing events
            DwollaWebhookEventType::TransferCreated 
            | DwollaWebhookEventType::CustomerTransferCreated 
            | DwollaWebhookEventType::BankTransferCreated => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing
            }
            
            // Payment failure events
            DwollaWebhookEventType::TransferFailed 
            | DwollaWebhookEventType::CustomerTransferFailed 
            | DwollaWebhookEventType::BankTransferFailed 
            | DwollaWebhookEventType::TransferReclaimed => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure
            }
            
            // Payment cancellation events
            DwollaWebhookEventType::TransferCancelled 
            | DwollaWebhookEventType::CustomerTransferCancelled 
            | DwollaWebhookEventType::BankTransferCancelled => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled
            }
            
            // Refund events (mass payments are typically refunds)
            DwollaWebhookEventType::MassPaymentCompleted => {
                api_models::webhooks::IncomingWebhookEvent::RefundSuccess
            }
            DwollaWebhookEventType::MassPaymentCreated => {
                api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing
            }
            DwollaWebhookEventType::MassPaymentCancelled => {
                api_models::webhooks::IncomingWebhookEvent::RefundFailure
            }
            
            // Customer verification events
            DwollaWebhookEventType::CustomerVerificationDocumentNeeded
            | DwollaWebhookEventType::CustomerReverificationNeeded => {
                api_models::webhooks::IncomingWebhookEvent::PaymentActionRequired
            }
            
            // All other events are not directly supported
            _ => api_models::webhooks::IncomingWebhookEvent::EventNotSupported,
        }
    }
    
    pub fn is_payment_related(&self) -> bool {
        matches!(self,
            DwollaWebhookEventType::TransferCreated
            | DwollaWebhookEventType::TransferCompleted
            | DwollaWebhookEventType::TransferFailed
            | DwollaWebhookEventType::TransferCancelled
            | DwollaWebhookEventType::TransferReclaimed
            | DwollaWebhookEventType::CustomerTransferCreated
            | DwollaWebhookEventType::CustomerTransferCompleted
            | DwollaWebhookEventType::CustomerTransferFailed
            | DwollaWebhookEventType::CustomerTransferCancelled
            | DwollaWebhookEventType::BankTransferCreated
            | DwollaWebhookEventType::BankTransferCompleted
            | DwollaWebhookEventType::BankTransferFailed
            | DwollaWebhookEventType::BankTransferCancelled
        )
    }
    
    pub fn is_refund_related(&self) -> bool {
        matches!(self,
            DwollaWebhookEventType::MassPaymentCreated
            | DwollaWebhookEventType::MassPaymentCompleted
            | DwollaWebhookEventType::MassPaymentCancelled
        )
    }
}

// Transfer Status Enum for webhook processing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DwollaTransferStatus {
    #[serde(rename = "processed")]
    Processed,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "reclaimed")]
    Reclaimed,
}

impl From<DwollaTransferStatus> for common_enums::AttemptStatus {
    fn from(item: DwollaTransferStatus) -> Self {
        match item {
            DwollaTransferStatus::Processed => Self::Charged,
            DwollaTransferStatus::Pending => Self::Pending,
            DwollaTransferStatus::Failed => Self::Failure,
            DwollaTransferStatus::Cancelled => Self::Voided,
            DwollaTransferStatus::Reclaimed => Self::Failure,
        }
    }
}

// Webhook event processing functions
pub fn extract_transfer_id_from_webhook(webhook_event: &DwollaWebhookEvent) -> Option<String> {
    // Extract transfer ID from the resource link
    webhook_event
        .links
        .resource
        .href
        .split('/')
        .last()
        .map(|id| id.to_string())
}

pub fn extract_customer_id_from_webhook(webhook_event: &DwollaWebhookEvent) -> Option<String> {
    // Extract customer ID from the customer link if present
    webhook_event
        .links
        .customer
        .as_ref()
        .and_then(|customer_link| {
            customer_link
                .href
                .split('/')
                .last()
                .map(|id| id.to_string())
        })
}

pub fn get_webhook_object_reference_id(
    webhook_event: &DwollaWebhookEvent,
) -> Result<api_models::webhooks::ObjectReferenceId, error_stack::Report<errors::ConnectorError>> {
    let event_type = DwollaWebhookEventType::from(webhook_event.topic.as_str());
    
    if event_type.is_payment_related() {
        // For payment-related events, use the transfer ID
        if let Some(transfer_id) = extract_transfer_id_from_webhook(webhook_event) {
            Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(transfer_id),
            ))
        } else {
            Err(errors::ConnectorError::WebhookReferenceIdNotFound.into())
        }
    } else if event_type.is_refund_related() {
        // For refund-related events, use the mass payment ID
        if let Some(refund_id) = extract_transfer_id_from_webhook(webhook_event) {
            Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::ConnectorRefundId(refund_id),
            ))
        } else {
            Err(errors::ConnectorError::WebhookReferenceIdNotFound.into())
        }
    } else {
        // For other events, we don't have a specific reference ID
        Err(errors::ConnectorError::WebhookReferenceIdNotFound.into())
    }
}

pub fn process_webhook_event_type(
    webhook_event: &DwollaWebhookEvent,
) -> api_models::webhooks::IncomingWebhookEvent {
    let event_type = DwollaWebhookEventType::from(webhook_event.topic.as_str());
    event_type.to_hyperswitch_event()
}

// Enhanced webhook response structure for detailed processing
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DwollaWebhookResponse {
    pub event: DwollaWebhookEvent,
    pub processed_event_type: api_models::webhooks::IncomingWebhookEvent,
    pub reference_id: Option<String>,
    pub status: Option<String>,
    pub amount: Option<DwollaAmount>,
}

impl TryFrom<&DwollaWebhookEvent> for DwollaWebhookResponse {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(webhook_event: &DwollaWebhookEvent) -> Result<Self, Self::Error> {
        let processed_event_type = process_webhook_event_type(webhook_event);
        let reference_id = extract_transfer_id_from_webhook(webhook_event);
        
        // Extract status and amount from embedded resource if available
        let (status, amount) = if let Some(embedded) = &webhook_event.embedded {
            if let Some(resource) = &embedded.resource {
                (resource.status.clone(), resource.amount.clone())
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        
        Ok(Self {
            event: webhook_event.clone(),
            processed_event_type,
            reference_id,
            status,
            amount,
        })
    }
}
