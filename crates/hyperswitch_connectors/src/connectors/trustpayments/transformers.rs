use common_enums::enums;
use common_utils::{pii::Email, request::Method, types::StringMinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{BankRedirectData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, CardData, RefundsRequestData, RouterData as RouterDataExt},
};

const TRUSTPAYMENTS_API_VERSION: &str = "1.00";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustpaymentsSettleStatus {
    #[serde(rename = "0")]
    PendingSettlement,
    #[serde(rename = "1")]
    Settled,
    #[serde(rename = "2")]
    ManualCapture,
    #[serde(rename = "3")]
    Voided,
    #[serde(rename = "10")]
    PendingSettlementRedirect,
    #[serde(rename = "100")]
    SettledRedirect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustpaymentsCredentialsOnFile {
    #[serde(rename = "0")]
    NoStoredCredentials,
    #[serde(rename = "1")]
    CardholderInitiatedTransaction,
    #[serde(rename = "2")]
    MerchantInitiatedTransaction,
}

impl TrustpaymentsCredentialsOnFile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoStoredCredentials => "0",
            Self::CardholderInitiatedTransaction => "1",
            Self::MerchantInitiatedTransaction => "2",
        }
    }
}

impl std::fmt::Display for TrustpaymentsCredentialsOnFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TrustpaymentsSettleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PendingSettlement => "0",
            Self::Settled => "1",
            Self::ManualCapture => "2",
            Self::Voided => "3",
            Self::PendingSettlementRedirect => "10",
            Self::SettledRedirect => "100",
        }
    }
}

impl std::fmt::Display for TrustpaymentsSettleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustpaymentsErrorCode {
    #[serde(rename = "0")]
    Success,

    #[serde(rename = "30000")]
    InvalidCredentials,
    #[serde(rename = "30001")]
    AuthenticationFailed,
    #[serde(rename = "30002")]
    InvalidSiteReference,
    #[serde(rename = "30003")]
    AccessDenied,
    #[serde(rename = "30004")]
    InvalidUsernameOrPassword,
    #[serde(rename = "30005")]
    AccountSuspended,
    #[serde(rename = "50000")]
    MissingRequiredField,
    #[serde(rename = "50001")]
    InvalidFieldFormat,
    #[serde(rename = "50002")]
    InvalidFieldValue,
    #[serde(rename = "50003")]
    FieldTooLong,
    #[serde(rename = "50004")]
    FieldTooShort,
    #[serde(rename = "50005")]
    InvalidCurrency,
    #[serde(rename = "50006")]
    InvalidAmount,
    #[serde(rename = "60000")]
    GeneralProcessingError,
    #[serde(rename = "60001")]
    SystemError,
    #[serde(rename = "60002")]
    CommunicationError,
    #[serde(rename = "60003")]
    Timeout,
    #[serde(rename = "60004")]
    Processing,
    #[serde(rename = "60005")]
    InvalidRequest,
    #[serde(rename = "60019")]
    NoSearchableFilter,
    #[serde(rename = "70000")]
    InvalidCardNumber,
    #[serde(rename = "70001")]
    InvalidExpiryDate,
    #[serde(rename = "70002")]
    InvalidSecurityCode,
    #[serde(rename = "70003")]
    InvalidCardType,
    #[serde(rename = "70004")]
    CardExpired,
    #[serde(rename = "70005")]
    InsufficientFunds,
    #[serde(rename = "70006")]
    CardDeclined,
    #[serde(rename = "70007")]
    CardRestricted,
    #[serde(rename = "70008")]
    InvalidMerchant,
    #[serde(rename = "70009")]
    TransactionNotPermitted,
    #[serde(rename = "70010")]
    ExceedsWithdrawalLimit,
    #[serde(rename = "70011")]
    SecurityViolation,
    #[serde(rename = "70012")]
    LostOrStolenCard,
    #[serde(rename = "70013")]
    SuspectedFraud,
    #[serde(rename = "70014")]
    ContactCardIssuer,
    #[serde(rename = "70015")]
    InvalidAmountValue,
    #[serde(untagged)]
    Unknown(String),
}

impl TrustpaymentsErrorCode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Success => "0",
            Self::InvalidCredentials => "30000",
            Self::AuthenticationFailed => "30001",
            Self::InvalidSiteReference => "30002",
            Self::AccessDenied => "30003",
            Self::InvalidUsernameOrPassword => "30004",
            Self::AccountSuspended => "30005",
            Self::MissingRequiredField => "50000",
            Self::InvalidFieldFormat => "50001",
            Self::InvalidFieldValue => "50002",
            Self::FieldTooLong => "50003",
            Self::FieldTooShort => "50004",
            Self::InvalidCurrency => "50005",
            Self::InvalidAmount => "50006",
            Self::GeneralProcessingError => "60000",
            Self::SystemError => "60001",
            Self::CommunicationError => "60002",
            Self::Timeout => "60003",
            Self::Processing => "60004",
            Self::InvalidRequest => "60005",
            Self::NoSearchableFilter => "60019",
            Self::InvalidCardNumber => "70000",
            Self::InvalidExpiryDate => "70001",
            Self::InvalidSecurityCode => "70002",
            Self::InvalidCardType => "70003",
            Self::CardExpired => "70004",
            Self::InsufficientFunds => "70005",
            Self::CardDeclined => "70006",
            Self::CardRestricted => "70007",
            Self::InvalidMerchant => "70008",
            Self::TransactionNotPermitted => "70009",
            Self::ExceedsWithdrawalLimit => "70010",
            Self::SecurityViolation => "70011",
            Self::LostOrStolenCard => "70012",
            Self::SuspectedFraud => "70013",
            Self::ContactCardIssuer => "70014",
            Self::InvalidAmountValue => "70015",
            Self::Unknown(code) => code,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    pub fn get_attempt_status(&self) -> common_enums::AttemptStatus {
        match self {
            // Success cases should be handled by get_payment_status() with settlestatus logic
            Self::Success => common_enums::AttemptStatus::Authorized,
            // Authentication and configuration errors
            Self::InvalidCredentials
            | Self::AuthenticationFailed
            | Self::InvalidSiteReference
            | Self::AccessDenied
            | Self::InvalidUsernameOrPassword
            | Self::AccountSuspended => common_enums::AttemptStatus::Failure,
            // Card-related and payment errors that should be treated as failures
            Self::InvalidCardNumber
            | Self::InvalidExpiryDate
            | Self::InvalidSecurityCode
            | Self::InvalidCardType
            | Self::CardExpired
            | Self::InsufficientFunds
            | Self::CardDeclined
            | Self::CardRestricted
            | Self::TransactionNotPermitted
            | Self::ExceedsWithdrawalLimit
            | Self::InvalidAmountValue => common_enums::AttemptStatus::Failure,
            // Processing states that should remain pending
            Self::Processing => common_enums::AttemptStatus::Pending,
            // Default fallback for unknown errors
            _ => common_enums::AttemptStatus::Pending,
        }
    }

    pub fn get_description(&self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::InvalidCredentials => "Invalid credentials",
            Self::AuthenticationFailed => "Authentication failed",
            Self::InvalidSiteReference => "Invalid site reference",
            Self::AccessDenied => "Access denied",
            Self::InvalidUsernameOrPassword => "Invalid username or password",
            Self::AccountSuspended => "Account suspended",
            Self::MissingRequiredField => "Missing required field",
            Self::InvalidFieldFormat => "Invalid field format",
            Self::InvalidFieldValue => "Invalid field value",
            Self::FieldTooLong => "Field value too long",
            Self::FieldTooShort => "Field value too short",
            Self::InvalidCurrency => "Invalid currency code",
            Self::InvalidAmount => "Invalid amount format",
            Self::GeneralProcessingError => "General processing error",
            Self::SystemError => "System error",
            Self::CommunicationError => "Communication error",
            Self::Timeout => "Request timeout",
            Self::Processing => "Transaction processing",
            Self::InvalidRequest => "Invalid request format",
            Self::NoSearchableFilter => "No searchable filter specified",
            Self::InvalidCardNumber => "Invalid card number",
            Self::InvalidExpiryDate => "Invalid expiry date",
            Self::InvalidSecurityCode => "Invalid security code",
            Self::InvalidCardType => "Invalid card type",
            Self::CardExpired => "Card expired",
            Self::InsufficientFunds => "Insufficient funds",
            Self::CardDeclined => "Card declined by issuer",
            Self::CardRestricted => "Card restricted",
            Self::InvalidMerchant => "Invalid merchant",
            Self::TransactionNotPermitted => "Transaction not permitted",
            Self::ExceedsWithdrawalLimit => "Exceeds withdrawal limit",
            Self::SecurityViolation => "Security violation",
            Self::LostOrStolenCard => "Lost or stolen card",
            Self::SuspectedFraud => "Suspected fraud",
            Self::ContactCardIssuer => "Contact card issuer",
            Self::InvalidAmountValue => "Invalid amount",
            Self::Unknown(_) => "Unknown error",
        }
    }
}

impl std::fmt::Display for TrustpaymentsErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct TrustpaymentsRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for TrustpaymentsRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}
#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsPaymentsRequest {
    pub alias: String,
    pub version: String,
    pub request: Vec<TrustpaymentsPaymentRequestData>,
}
#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsPaymentRequestData {
    // Common fields for all payment types
    pub sitereference: String,
    pub requesttypedescriptions: Vec<String>,
    pub accounttypedescription: String,
    pub currencyiso3a: String,
    pub baseamount: String,
    pub orderreference: String,

    // Optional fields based on payment method
    pub paymenttypedescription: Option<String>,

    // Card-specific fields
    pub pan: Option<cards::CardNumber>,
    pub expirydate: Option<Secret<String>>,
    pub securitycode: Option<Secret<String>>,
    pub credentialsonfile: Option<String>,
    pub settlestatus: Option<String>,

    // Redirect-specific fields
    pub returnurl: Option<String>,
    pub successfulurlredirect: Option<String>,
    pub errorurlredirect: Option<String>,

    // Billing fields (common across payment types)
    pub billingfirstname: Option<String>,
    pub billinglastname: Option<String>,
    pub billingcountryiso2a: Option<String>,
    pub billingemail: Option<Email>,

    // Application type for specific payment methods
    pub applicationtype: Option<String>,
}

impl TryFrom<&TrustpaymentsRouterData<&PaymentsAuthorizeRouterData>>
    for TrustpaymentsPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TrustpaymentsRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = TrustpaymentsAuthType::try_from(&item.router_data.connector_auth_type)?;

        if matches!(
            item.router_data.auth_type,
            enums::AuthenticationType::ThreeDs
        ) {
            return Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("trustpayments"),
            )
            .into());
        }

        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                Self::build_card_request(item, &auth, &req_card)
            }
            PaymentMethodData::BankRedirect(bank_redirect_data) => {
                match bank_redirect_data {
                    BankRedirectData::Eps { .. } => {
                        Self::build_eps_request(item, &auth)
                    }
                    BankRedirectData::Trustly { .. } => {
                        Self::build_trustly_request(item, &auth)
                    }
                    _ => Err(errors::ConnectorError::NotImplemented(
                        "Bank redirect method not supported".to_string(),
                    ).into()),
                }
            }
            PaymentMethodData::Wallet(wallet_data) => {
                match wallet_data {
                    hyperswitch_domain_models::payment_method_data::WalletData::AliPayRedirect { .. } => {
                        Self::build_alipay_request(item, &auth)
                    }
                    hyperswitch_domain_models::payment_method_data::WalletData::Paysera(_) => {
                        Self::build_paysera_request(item, &auth)
                    }
                    _ => Err(errors::ConnectorError::NotImplemented(
                        "Wallet method not supported".to_string(),
                    ).into()),
                }
            }
            PaymentMethodData::BankTransfer(bank_transfer_data) => {
                match &*bank_transfer_data {
                    hyperswitch_domain_models::payment_method_data::BankTransferData::SepaBankTransfer { .. } => {
                        Self::build_sepa_request(item, &auth)
                    }
                    _ => Err(errors::ConnectorError::NotImplemented(
                        "Bank transfer method not supported".to_string(),
                    ).into()),
                }
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method not supported".to_string(),
            ).into()),
        }
    }
}

impl TrustpaymentsPaymentsRequest {
    fn build_card_request(
        item: &TrustpaymentsRouterData<&PaymentsAuthorizeRouterData>,
        auth: &TrustpaymentsAuthType,
        card: &hyperswitch_domain_models::payment_method_data::Card,
    ) -> Result<Self, error_stack::Report<errors::ConnectorError>> {
        let request_types = match item.router_data.request.capture_method {
            Some(common_enums::CaptureMethod::Automatic) | None => vec!["AUTH".to_string()],
            Some(common_enums::CaptureMethod::Manual) => vec!["AUTH".to_string()],
            Some(common_enums::CaptureMethod::ManualMultiple)
            | Some(common_enums::CaptureMethod::Scheduled)
            | Some(common_enums::CaptureMethod::SequentialAutomatic) => {
                return Err(errors::ConnectorError::NotSupported {
                    message: "Capture method not supported by TrustPayments".to_string(),
                    connector: "TrustPayments",
                }
                .into());
            }
        };

        Ok(Self {
            alias: auth.username.clone().expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsPaymentRequestData {
                sitereference: auth.site_reference.clone().expose(),
                requesttypedescriptions: request_types,
                accounttypedescription: "ECOM".to_string(),
                currencyiso3a: item.router_data.request.currency.to_string(),
                baseamount: item.amount.to_string(),
                orderreference: item.router_data.connector_request_reference_id.clone(),

                // Card-specific fields
                paymenttypedescription: None,
                pan: Some(card.card_number.clone()),
                expirydate: Some(
                    card.get_card_expiry_month_year_2_digit_with_delimiter("/".to_string())?,
                ),
                securitycode: Some(card.card_cvc.clone()),
                credentialsonfile: Some(
                    TrustpaymentsCredentialsOnFile::CardholderInitiatedTransaction.to_string(),
                ),
                settlestatus: Some(match item.router_data.request.capture_method {
                    Some(common_enums::CaptureMethod::Manual) => {
                        TrustpaymentsSettleStatus::ManualCapture
                            .as_str()
                            .to_string()
                    }
                    Some(common_enums::CaptureMethod::Automatic) | None => {
                        TrustpaymentsSettleStatus::PendingSettlement
                            .as_str()
                            .to_string()
                    }
                    _ => TrustpaymentsSettleStatus::PendingSettlement
                        .as_str()
                        .to_string(),
                }),

                // Not used for cards
                returnurl: None,
                successfulurlredirect: None,
                errorurlredirect: None,

                // Billing fields
                billingfirstname: item
                    .router_data
                    .get_optional_billing_first_name()
                    .map(|name| name.expose()),
                billinglastname: item
                    .router_data
                    .get_optional_billing_last_name()
                    .map(|name| name.expose()),
                billingcountryiso2a: item
                    .router_data
                    .get_optional_billing_country()
                    .map(|country| country.to_string()),
                billingemail: item.router_data.get_optional_billing_email(),

                // Not used for cards
                applicationtype: None,
            }],
        })
    }

    fn build_eps_request(
        item: &TrustpaymentsRouterData<&PaymentsAuthorizeRouterData>,
        auth: &TrustpaymentsAuthType,
    ) -> Result<Self, error_stack::Report<errors::ConnectorError>> {
        Ok(Self {
            alias: auth.username.clone().expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsPaymentRequestData {
                sitereference: auth.site_reference.clone().expose(),
                requesttypedescriptions: vec!["AUTH".to_string()],
                accounttypedescription: "ECOM".to_string(),
                currencyiso3a: item.router_data.request.currency.to_string(),
                baseamount: item.amount.to_string(),
                orderreference: item.router_data.connector_request_reference_id.clone(),

                // EPS-specific fields
                paymenttypedescription: Some("EPS".to_string()),
                returnurl: item.router_data.request.router_return_url.clone(),
                successfulurlredirect: item.router_data.request.router_return_url.clone(),
                errorurlredirect: item.router_data.request.router_return_url.clone(),

                // Billing fields
                billingfirstname: item
                    .router_data
                    .get_optional_billing_first_name()
                    .map(|name| name.expose()),
                billinglastname: item
                    .router_data
                    .get_optional_billing_last_name()
                    .map(|name| name.expose()),
                billingcountryiso2a: item
                    .router_data
                    .get_optional_billing_country()
                    .map(|country| country.to_string()),
                billingemail: item.router_data.get_optional_billing_email(),

                // Not used for EPS
                pan: None,
                expirydate: None,
                securitycode: None,
                credentialsonfile: None,
                settlestatus: None,
                applicationtype: None,
            }],
        })
    }

    fn build_trustly_request(
        item: &TrustpaymentsRouterData<&PaymentsAuthorizeRouterData>,
        auth: &TrustpaymentsAuthType,
    ) -> Result<Self, error_stack::Report<errors::ConnectorError>> {
        Ok(Self {
            alias: auth.username.clone().expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsPaymentRequestData {
                sitereference: auth.site_reference.clone().expose(),
                requesttypedescriptions: vec!["AUTH".to_string()],
                accounttypedescription: "ECOM".to_string(),
                currencyiso3a: item.router_data.request.currency.to_string(),
                baseamount: item.amount.to_string(),
                orderreference: item.router_data.connector_request_reference_id.clone(),

                // Trustly-specific fields
                paymenttypedescription: Some("TRUSTLY".to_string()),
                returnurl: item.router_data.request.router_return_url.clone(),
                successfulurlredirect: None,
                errorurlredirect: None,

                // Billing fields
                billingfirstname: item
                    .router_data
                    .get_optional_billing_first_name()
                    .map(|name| name.expose()),
                billinglastname: item
                    .router_data
                    .get_optional_billing_last_name()
                    .map(|name| name.expose()),
                billingcountryiso2a: item
                    .router_data
                    .get_optional_billing_country()
                    .map(|country| country.to_string()),
                billingemail: item.router_data.get_optional_billing_email(),

                // Not used for Trustly
                pan: None,
                expirydate: None,
                securitycode: None,
                credentialsonfile: None,
                settlestatus: None,
                applicationtype: None,
            }],
        })
    }

    fn build_alipay_request(
        item: &TrustpaymentsRouterData<&PaymentsAuthorizeRouterData>,
        auth: &TrustpaymentsAuthType,
    ) -> Result<Self, error_stack::Report<errors::ConnectorError>> {
        Ok(Self {
            alias: auth.username.clone().expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsPaymentRequestData {
                sitereference: auth.site_reference.clone().expose(),
                requesttypedescriptions: vec!["AUTH".to_string()],
                accounttypedescription: "ECOM".to_string(),
                currencyiso3a: item.router_data.request.currency.to_string(),
                baseamount: item.amount.to_string(),
                orderreference: item.router_data.connector_request_reference_id.clone(),

                // Alipay-specific fields
                paymenttypedescription: Some("ALIPAY".to_string()),
                returnurl: item.router_data.request.router_return_url.clone(),
                successfulurlredirect: None,
                errorurlredirect: None,

                // Billing fields
                billingfirstname: None,
                billinglastname: None,
                billingcountryiso2a: item
                    .router_data
                    .get_optional_billing_country()
                    .map(|country| country.to_string())
                    .or_else(|| Some("CN".to_string())),
                billingemail: item.router_data.get_optional_billing_email(),

                // Not used for Alipay
                pan: None,
                expirydate: None,
                securitycode: None,
                credentialsonfile: None,
                settlestatus: None,
                applicationtype: None,
            }],
        })
    }

    fn build_paysera_request(
        item: &TrustpaymentsRouterData<&PaymentsAuthorizeRouterData>,
        auth: &TrustpaymentsAuthType,
    ) -> Result<Self, error_stack::Report<errors::ConnectorError>> {
        Ok(Self {
            alias: auth.username.clone().expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsPaymentRequestData {
                sitereference: auth.site_reference.clone().expose(),
                requesttypedescriptions: vec!["AUTH".to_string()],
                accounttypedescription: "ECOM".to_string(),
                currencyiso3a: item.router_data.request.currency.to_string(),
                baseamount: item.amount.to_string(),
                orderreference: item.router_data.connector_request_reference_id.clone(),

                // Paysera-specific fields
                paymenttypedescription: Some("PAYSERA".to_string()),
                returnurl: item.router_data.request.router_return_url.clone(),
                successfulurlredirect: item.router_data.request.router_return_url.clone(),
                errorurlredirect: item.router_data.request.router_return_url.clone(),

                // Billing fields
                billingfirstname: item
                    .router_data
                    .get_optional_billing_first_name()
                    .map(|name| name.expose()),
                billinglastname: item
                    .router_data
                    .get_optional_billing_last_name()
                    .map(|name| name.expose()),
                billingcountryiso2a: item
                    .router_data
                    .get_optional_billing_country()
                    .map(|country| country.to_string()),
                billingemail: item.router_data.get_optional_billing_email(),

                // Not used for Paysera
                pan: None,
                expirydate: None,
                securitycode: None,
                credentialsonfile: None,
                settlestatus: None,
                applicationtype: None,
            }],
        })
    }

    fn build_sepa_request(
        item: &TrustpaymentsRouterData<&PaymentsAuthorizeRouterData>,
        auth: &TrustpaymentsAuthType,
    ) -> Result<Self, error_stack::Report<errors::ConnectorError>> {
        Ok(Self {
            alias: auth.username.clone().expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsPaymentRequestData {
                sitereference: auth.site_reference.clone().expose(),
                requesttypedescriptions: vec!["AUTH".to_string()],
                accounttypedescription: "ECOM".to_string(),
                currencyiso3a: item.router_data.request.currency.to_string(),
                baseamount: item.amount.to_string(),
                orderreference: item.router_data.connector_request_reference_id.clone(),

                // SEPA-specific fields
                paymenttypedescription: Some("SEPA".to_string()),
                returnurl: None,
                successfulurlredirect: None,
                errorurlredirect: None,

                // Billing fields
                billingfirstname: item
                    .router_data
                    .get_optional_billing_first_name()
                    .map(|name| name.expose()),
                billinglastname: item
                    .router_data
                    .get_optional_billing_last_name()
                    .map(|name| name.expose()),
                billingcountryiso2a: item
                    .router_data
                    .get_optional_billing_country()
                    .map(|country| country.to_string()),
                billingemail: item.router_data.get_optional_billing_email(),

                // Not used for SEPA
                pan: None,
                expirydate: None,
                securitycode: None,
                credentialsonfile: None,
                settlestatus: None,
                applicationtype: None,
            }],
        })
    }
}

pub struct TrustpaymentsAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
    pub(super) site_reference: Secret<String>,
}

impl TrustpaymentsAuthType {
    pub fn get_basic_auth_header(&self) -> String {
        use base64::Engine;
        let credentials = format!(
            "{}:{}",
            self.username.clone().expose(),
            self.password.clone().expose()
        );
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {encoded}")
    }
}

impl TryFrom<&ConnectorAuthType> for TrustpaymentsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                username: api_key.to_owned(),
                password: key1.to_owned(),
                site_reference: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustpaymentsPaymentsResponse {
    #[serde(alias = "response")]
    pub responses: Vec<TrustpaymentsPaymentResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustpaymentsPaymentResponseData {
    pub errorcode: TrustpaymentsErrorCode,
    pub errormessage: String,
    pub authcode: Option<String>,
    pub baseamount: Option<StringMinorUnit>,
    pub currencyiso3a: Option<String>,
    pub transactionreference: Option<String>,
    pub settlestatus: Option<TrustpaymentsSettleStatus>,
    pub requesttypedescription: String,
    pub securityresponsesecuritycode: Option<String>,
    pub redirecturl: Option<String>,
    pub records: Option<Vec<TrustpaymentsPaymentRecords>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustpaymentsPaymentRecords {
    pub settlestatus: Option<TrustpaymentsSettleStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustpaymentsAlipayResponse {
    #[serde(alias = "response")]
    pub responses: Vec<TrustpaymentsAlipayResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustpaymentsAlipayResponseData {
    pub errorcode: TrustpaymentsErrorCode,
    pub errormessage: String,
    pub transactionreference: Option<String>,
    pub requesttypedescription: String,
    pub redirecturl: Option<String>,
    pub merchantname: Option<String>,
    pub paymenttypedescription: Option<String>,
    pub orderreference: Option<String>,
    pub settleduedate: Option<String>,
    pub baseamount: Option<String>,
    pub transactionstartedtimestamp: Option<String>,
    pub settlestatus: Option<String>,
    pub accounttypedescription: Option<String>,
    pub operatorname: Option<String>,
    pub livestatus: Option<String>,
    pub currencyiso3a: Option<String>,
    #[serde(rename = "partner id")]
    pub partner_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustpaymentsPayseraResponse {
    #[serde(alias = "response")]
    pub responses: Vec<TrustpaymentsPayseraResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustpaymentsPayseraResponseData {
    pub errorcode: TrustpaymentsErrorCode,
    pub errormessage: String,
    pub transactionreference: Option<String>,
    pub requesttypedescription: String,
    pub redirecturl: Option<String>,
    pub merchantname: Option<String>,
    pub paymenttypedescription: Option<String>,
    pub orderreference: Option<String>,
    pub settleduedate: Option<String>,
    pub baseamount: Option<String>,
    pub transactionstartedtimestamp: Option<String>,
    pub settlestatus: Option<String>,
    pub accounttypedescription: Option<String>,
    pub operatorname: Option<String>,
    pub livestatus: Option<String>,
    pub currencyiso3a: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustpaymentsSepaResponse {
    #[serde(alias = "response")]
    pub responses: Vec<TrustpaymentsSepaResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustpaymentsSepaResponseData {
    pub errorcode: TrustpaymentsErrorCode,
    pub errormessage: String,
    pub transactionreference: Option<String>,
    pub requesttypedescription: String,
    pub redirecturl: Option<String>,
    pub merchantname: Option<String>,
    pub paymenttypedescription: Option<String>,
    pub orderreference: Option<String>,
    pub settleduedate: Option<String>,
    pub baseamount: Option<String>,
    pub transactionstartedtimestamp: Option<String>,
    pub settlestatus: Option<String>,
    pub accounttypedescription: Option<String>,
    pub operatorname: Option<String>,
    pub livestatus: Option<String>,
    pub currencyiso3a: Option<String>,
}

impl TrustpaymentsAlipayResponseData {
    pub fn get_alipay_status(&self) -> common_enums::AttemptStatus {
        match self.errorcode {
            TrustpaymentsErrorCode::Success => {
                if self.redirecturl.is_some() {
                    common_enums::AttemptStatus::AuthenticationPending
                } else {
                    common_enums::AttemptStatus::Authorized
                }
            }
            _ => self.errorcode.get_attempt_status(),
        }
    }
}

impl TrustpaymentsSepaResponseData {
    pub fn get_sepa_status(&self) -> common_enums::AttemptStatus {
        match self.errorcode {
            TrustpaymentsErrorCode::Success => {
                if self.redirecturl.is_some() {
                    common_enums::AttemptStatus::AuthenticationPending
                } else {
                    common_enums::AttemptStatus::Authorized
                }
            }
            _ => self.errorcode.get_attempt_status(),
        }
    }
}

impl TrustpaymentsPayseraResponseData {
    pub fn get_paysera_status(&self) -> common_enums::AttemptStatus {
        match self.errorcode {
            TrustpaymentsErrorCode::Success => {
                if self.redirecturl.is_some() {
                    common_enums::AttemptStatus::AuthenticationPending
                } else if self.requesttypedescription == "TRANSACTIONQUERY"
                    && self.transactionreference.is_none()
                    && self.redirecturl.is_none()
                {
                    // For Paysera sync queries, when transactionreference and redirecturl are null,
                    // check if we have any indication of completion (timestamp, settlestatus, etc.)
                    if self.transactionstartedtimestamp.is_some()
                        || self.settlestatus.is_some()
                        || self.baseamount.is_some()
                    {
                        // Payment has been processed/completed
                        common_enums::AttemptStatus::Charged
                    } else {
                        // Payment is still pending (no processing indicators)
                        common_enums::AttemptStatus::AuthenticationPending
                    }
                } else {
                    common_enums::AttemptStatus::Authorized
                }
            }
            _ => self.errorcode.get_attempt_status(),
        }
    }
}

impl TrustpaymentsPaymentResponseData {
    fn map_settlestatus_to_attempt_status(
        &self,
        default_for_none: common_enums::AttemptStatus,
    ) -> common_enums::AttemptStatus {
        match &self.settlestatus {
            Some(TrustpaymentsSettleStatus::PendingSettlement) => {
                // settlestatus "0" = Authorized. The payment has been authorized, no further action required
                common_enums::AttemptStatus::Charged
            }
            Some(TrustpaymentsSettleStatus::Settled) => {
                // settlestatus "1" = Authorized. Same as status 0, the payment has been authorized
                common_enums::AttemptStatus::Charged
            }
            Some(TrustpaymentsSettleStatus::ManualCapture) => {
                // settlestatus "2" = Suspended. Funds are not scheduled for settlement
                common_enums::AttemptStatus::Authorized
            }
            Some(TrustpaymentsSettleStatus::Voided) => {
                // settlestatus "3" = Cancelled. The payment has been declined, encountered an error, or was subsequently cancelled
                common_enums::AttemptStatus::Voided
            }
            Some(TrustpaymentsSettleStatus::PendingSettlementRedirect) => {
                // settlestatus "10" = Pending settlement. The initial status after a successful AUTH request, indicating funds have not yet been settled
                common_enums::AttemptStatus::Authorizing
            }
            Some(TrustpaymentsSettleStatus::SettledRedirect) => {
                // settlestatus "100" = Settled. The funds have been successfully transferred to your account
                common_enums::AttemptStatus::Charged
            }
            None => default_for_none,
        }
    }

    pub fn get_payment_status(&self) -> common_enums::AttemptStatus {
        match self.errorcode {
            TrustpaymentsErrorCode::Success => {
                if self.redirecturl.is_some() {
                    // For redirect-based payments (like EPS), return AuthenticationPending
                    // This will trigger Hyperswitch to populate the next_action field
                    common_enums::AttemptStatus::AuthenticationPending
                } else if self.authcode.is_some() {
                    // For card payments with authcode
                    self.map_settlestatus_to_attempt_status(common_enums::AttemptStatus::Authorized)
                } else {
                    common_enums::AttemptStatus::Failure
                }
            }
            _ => self.errorcode.get_attempt_status(),
        }
    }

    pub fn get_payment_status_for_sync(&self) -> common_enums::AttemptStatus {
        let status = match self.errorcode {
            TrustpaymentsErrorCode::Success => {
                if self.requesttypedescription == "TRANSACTIONQUERY"
                    && self.authcode.is_none()
                    && self.records.is_none()
                    && self.transactionreference.is_none()
                {
                    common_enums::AttemptStatus::Charged
                } else if self.authcode.is_some() {
                    match &self.settlestatus {
                        Some(TrustpaymentsSettleStatus::PendingSettlement) => {
                            common_enums::AttemptStatus::Authorized
                        }
                        Some(TrustpaymentsSettleStatus::Settled) => {
                            common_enums::AttemptStatus::Charged
                        }
                        Some(TrustpaymentsSettleStatus::ManualCapture) => {
                            common_enums::AttemptStatus::Authorized
                        }
                        Some(TrustpaymentsSettleStatus::Voided) => {
                            common_enums::AttemptStatus::Voided
                        }
                        Some(TrustpaymentsSettleStatus::PendingSettlementRedirect) => {
                            common_enums::AttemptStatus::Authorizing
                        }
                        Some(TrustpaymentsSettleStatus::SettledRedirect) => {
                            common_enums::AttemptStatus::Charged
                        }
                        None => common_enums::AttemptStatus::Authorized,
                    }
                } else if self.requesttypedescription == "TRANSACTIONQUERY"
                    && self.authcode.is_none()
                    && self.records.is_some()
                {
                    match self
                        .records
                        .as_ref()
                        .and_then(|records| records.first())
                        .and_then(|record| record.settlestatus.as_ref())
                    {
                        Some(TrustpaymentsSettleStatus::PendingSettlement) => {
                            common_enums::AttemptStatus::Authorized
                        }
                        Some(TrustpaymentsSettleStatus::Settled) => {
                            common_enums::AttemptStatus::Charged
                        }
                        Some(TrustpaymentsSettleStatus::ManualCapture) => {
                            common_enums::AttemptStatus::Authorized
                        }
                        Some(TrustpaymentsSettleStatus::Voided) => {
                            common_enums::AttemptStatus::Voided
                        }
                        Some(TrustpaymentsSettleStatus::PendingSettlementRedirect) => {
                            common_enums::AttemptStatus::Authorizing
                        }
                        Some(TrustpaymentsSettleStatus::SettledRedirect) => {
                            common_enums::AttemptStatus::Charged
                        }
                        None => common_enums::AttemptStatus::Authorized,
                    }
                } else {
                    common_enums::AttemptStatus::Pending
                }
            }
            _ => self.errorcode.get_attempt_status(),
        };

        status
    }

    pub fn get_error_message(&self) -> String {
        if self.errorcode.is_success() {
            "Success".to_string()
        } else {
            format!("Error {}: {}", self.errorcode, self.errormessage)
        }
    }

    pub fn get_error_reason(&self) -> Option<String> {
        if !self.errorcode.is_success() {
            Some(self.errorcode.get_description().to_string())
        } else {
            None
        }
    }
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Authorize,
            TrustpaymentsPaymentsResponse,
            hyperswitch_domain_models::router_request_types::PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    >
    for RouterData<
        hyperswitch_domain_models::router_flow_types::payments::Authorize,
        hyperswitch_domain_models::router_request_types::PaymentsAuthorizeData,
        PaymentsResponseData,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Authorize,
            TrustpaymentsPaymentsResponse,
            hyperswitch_domain_models::router_request_types::PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response_data = item
            .response
            .responses
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

        let status: enums::AttemptStatus = response_data.get_payment_status();
        let transaction_id = response_data
            .transactionreference
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        if !response_data.errorcode.is_success() {
            let _error_response = TrustpaymentsErrorResponse::from(response_data.clone());
            return Ok(Self {
                status,
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: response_data.errorcode.to_string(),
                    message: response_data.errormessage.clone(),
                    reason: response_data.get_error_reason(),
                    status_code: item.http_code,
                    attempt_status: Some(status),
                    connector_transaction_id: response_data.transactionreference.clone(),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            });
        }

        let redirection_data = if let Some(redirect_url) = &response_data.redirecturl {
            match url::Url::parse(redirect_url) {
                Ok(parsed_url) => Some(
                    hyperswitch_domain_models::router_response_types::RedirectForm::from((
                        parsed_url,
                        Method::Get,
                    )),
                ),
                Err(_) => None,
            }
        } else {
            None
        };

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::PSync,
            TrustpaymentsPaymentsResponse,
            hyperswitch_domain_models::router_request_types::PaymentsSyncData,
            PaymentsResponseData,
        >,
    >
    for RouterData<
        hyperswitch_domain_models::router_flow_types::payments::PSync,
        hyperswitch_domain_models::router_request_types::PaymentsSyncData,
        PaymentsResponseData,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::PSync,
            TrustpaymentsPaymentsResponse,
            hyperswitch_domain_models::router_request_types::PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response_data = item
            .response
            .responses
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

        let status = response_data.get_payment_status_for_sync();

        let transaction_id = item
            .data
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

        if !response_data.errorcode.is_success() {
            return Ok(Self {
                status,
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: response_data.errorcode.to_string(),
                    message: response_data.errormessage.clone(),
                    reason: response_data.get_error_reason(),
                    status_code: item.http_code,
                    attempt_status: Some(status),
                    connector_transaction_id: Some(transaction_id.clone()),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            });
        }

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Capture,
            TrustpaymentsPaymentsResponse,
            hyperswitch_domain_models::router_request_types::PaymentsCaptureData,
            PaymentsResponseData,
        >,
    >
    for RouterData<
        hyperswitch_domain_models::router_flow_types::payments::Capture,
        hyperswitch_domain_models::router_request_types::PaymentsCaptureData,
        PaymentsResponseData,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Capture,
            TrustpaymentsPaymentsResponse,
            hyperswitch_domain_models::router_request_types::PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response_data = item
            .response
            .responses
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

        let transaction_id = item.data.request.connector_transaction_id.clone();
        let status = if response_data.errorcode.is_success() {
            common_enums::AttemptStatus::Charged
        } else {
            response_data.get_payment_status()
        };

        if !response_data.errorcode.is_success() {
            return Ok(Self {
                status,
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: response_data.errorcode.to_string(),
                    message: response_data.errormessage.clone(),
                    reason: response_data.get_error_reason(),
                    status_code: item.http_code,
                    attempt_status: Some(status),
                    connector_transaction_id: Some(transaction_id.clone()),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            });
        }

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Void,
            TrustpaymentsPaymentsResponse,
            hyperswitch_domain_models::router_request_types::PaymentsCancelData,
            PaymentsResponseData,
        >,
    >
    for RouterData<
        hyperswitch_domain_models::router_flow_types::payments::Void,
        hyperswitch_domain_models::router_request_types::PaymentsCancelData,
        PaymentsResponseData,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Void,
            TrustpaymentsPaymentsResponse,
            hyperswitch_domain_models::router_request_types::PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response_data = item
            .response
            .responses
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

        let transaction_id = item.data.request.connector_transaction_id.clone();
        let status = if response_data.errorcode.is_success() {
            common_enums::AttemptStatus::Voided
        } else {
            response_data.get_payment_status()
        };

        if !response_data.errorcode.is_success() {
            return Ok(Self {
                status,
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: response_data.errorcode.to_string(),
                    message: response_data.errormessage.clone(),
                    reason: response_data.get_error_reason(),
                    status_code: item.http_code,
                    attempt_status: Some(status),
                    connector_transaction_id: Some(transaction_id.clone()),
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            });
        }

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(transaction_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}
#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsCaptureRequest {
    pub alias: String,
    pub version: String,
    pub request: Vec<TrustpaymentsCaptureRequestData>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsCaptureRequestData {
    pub requesttypedescriptions: Vec<String>,
    pub filter: TrustpaymentsFilter,
    pub updates: TrustpaymentsCaptureUpdates,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsCaptureUpdates {
    pub settlestatus: TrustpaymentsSettleStatus,
}

impl TryFrom<&TrustpaymentsRouterData<&PaymentsCaptureRouterData>> for TrustpaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TrustpaymentsRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = TrustpaymentsAuthType::try_from(&item.router_data.connector_auth_type)?;

        let transaction_reference = item.router_data.request.connector_transaction_id.clone();

        Ok(Self {
            alias: auth.username.expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsCaptureRequestData {
                requesttypedescriptions: vec!["TRANSACTIONUPDATE".to_string()],
                filter: TrustpaymentsFilter {
                    sitereference: vec![TrustpaymentsFilterValue {
                        value: auth.site_reference.expose(),
                    }],
                    transactionreference: vec![TrustpaymentsFilterValue {
                        value: transaction_reference,
                    }],
                },
                updates: TrustpaymentsCaptureUpdates {
                    settlestatus: TrustpaymentsSettleStatus::PendingSettlement,
                },
            }],
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsVoidRequest {
    pub alias: String,
    pub version: String,
    pub request: Vec<TrustpaymentsVoidRequestData>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsVoidRequestData {
    pub requesttypedescriptions: Vec<String>,
    pub filter: TrustpaymentsFilter,
    pub updates: TrustpaymentsVoidUpdates,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsVoidUpdates {
    pub settlestatus: TrustpaymentsSettleStatus,
}

impl TryFrom<&PaymentsCancelRouterData> for TrustpaymentsVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = TrustpaymentsAuthType::try_from(&item.connector_auth_type)?;

        let transaction_reference = item.request.connector_transaction_id.clone();

        Ok(Self {
            alias: auth.username.expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsVoidRequestData {
                requesttypedescriptions: vec!["TRANSACTIONUPDATE".to_string()],
                filter: TrustpaymentsFilter {
                    sitereference: vec![TrustpaymentsFilterValue {
                        value: auth.site_reference.expose(),
                    }],
                    transactionreference: vec![TrustpaymentsFilterValue {
                        value: transaction_reference,
                    }],
                },
                updates: TrustpaymentsVoidUpdates {
                    settlestatus: TrustpaymentsSettleStatus::Voided,
                },
            }],
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsRefundRequest {
    pub alias: String,
    pub version: String,
    pub request: Vec<TrustpaymentsRefundRequestData>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsRefundRequestData {
    pub requesttypedescriptions: Vec<String>,
    pub sitereference: String,
    pub parenttransactionreference: String,
    pub baseamount: StringMinorUnit,
    pub currencyiso3a: String,
}

impl<F> TryFrom<&TrustpaymentsRouterData<&RefundsRouterData<F>>> for TrustpaymentsRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TrustpaymentsRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let auth = TrustpaymentsAuthType::try_from(&item.router_data.connector_auth_type)?;

        let parent_transaction_reference =
            item.router_data.request.connector_transaction_id.clone();

        Ok(Self {
            alias: auth.username.expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsRefundRequestData {
                requesttypedescriptions: vec!["REFUND".to_string()],
                sitereference: auth.site_reference.expose(),
                parenttransactionreference: parent_transaction_reference,
                baseamount: item.amount.clone(),
                currencyiso3a: item.router_data.request.currency.to_string(),
            }],
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsSyncRequest {
    pub alias: String,
    pub version: String,
    pub request: Vec<TrustpaymentsSyncRequestData>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsSyncRequestData {
    pub requesttypedescriptions: Vec<String>,
    pub filter: TrustpaymentsFilter,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsFilter {
    pub sitereference: Vec<TrustpaymentsFilterValue>,
    pub transactionreference: Vec<TrustpaymentsFilterValue>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsFilterValue {
    pub value: String,
}

impl TryFrom<&PaymentsSyncRouterData> for TrustpaymentsSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = TrustpaymentsAuthType::try_from(&item.connector_auth_type)?;

        let transaction_reference = item
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

        Ok(Self {
            alias: auth.username.expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsSyncRequestData {
                requesttypedescriptions: vec!["TRANSACTIONQUERY".to_string()],
                filter: TrustpaymentsFilter {
                    sitereference: vec![TrustpaymentsFilterValue {
                        value: auth.site_reference.expose(),
                    }],
                    transactionreference: vec![TrustpaymentsFilterValue {
                        value: transaction_reference,
                    }],
                },
            }],
        })
    }
}

pub type TrustpaymentsRefundSyncRequest = TrustpaymentsSyncRequest;

impl TryFrom<&RefundSyncRouterData> for TrustpaymentsRefundSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth = TrustpaymentsAuthType::try_from(&item.connector_auth_type)?;

        let refund_transaction_reference = item
            .request
            .get_connector_refund_id()
            .change_context(errors::ConnectorError::MissingConnectorRefundID)?;

        Ok(Self {
            alias: auth.username.expose(),
            version: TRUSTPAYMENTS_API_VERSION.to_string(),
            request: vec![TrustpaymentsSyncRequestData {
                requesttypedescriptions: vec!["TRANSACTIONQUERY".to_string()],
                filter: TrustpaymentsFilter {
                    sitereference: vec![TrustpaymentsFilterValue {
                        value: auth.site_reference.expose(),
                    }],
                    transactionreference: vec![TrustpaymentsFilterValue {
                        value: refund_transaction_reference,
                    }],
                },
            }],
        })
    }
}

pub type RefundResponse = TrustpaymentsPaymentsResponse;

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let response_data = item
            .response
            .responses
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

        let refund_id = response_data
            .transactionreference
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let refund_status = response_data.get_refund_status();

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: refund_id,
                refund_status,
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
        let response_data = item
            .response
            .responses
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

        let refund_id = response_data
            .transactionreference
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let refund_status = response_data.get_refund_status();

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: refund_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsTokenizationRequest {
    pub alias: String,
    pub version: String,
    pub request: Vec<TrustpaymentsTokenizationRequestData>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TrustpaymentsTokenizationRequestData {
    pub accounttypedescription: String,
    pub requesttypedescriptions: Vec<String>,
    pub sitereference: String,
    pub pan: cards::CardNumber,
    pub expirydate: Secret<String>,
    pub securitycode: Secret<String>,
    pub credentialsonfile: String,
}

impl
    TryFrom<
        &RouterData<
            hyperswitch_domain_models::router_flow_types::payments::PaymentMethodToken,
            hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    > for TrustpaymentsTokenizationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RouterData<
            hyperswitch_domain_models::router_flow_types::payments::PaymentMethodToken,
            hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let auth = TrustpaymentsAuthType::try_from(&item.connector_auth_type)?;

        match &item.request.payment_method_data {
            PaymentMethodData::Card(card_data) => Ok(Self {
                alias: auth.username.expose(),
                version: TRUSTPAYMENTS_API_VERSION.to_string(),
                request: vec![TrustpaymentsTokenizationRequestData {
                    accounttypedescription: "ECOM".to_string(),
                    requesttypedescriptions: vec!["ACCOUNTCHECK".to_string()],
                    sitereference: auth.site_reference.expose(),
                    pan: card_data.card_number.clone(),
                    expirydate: card_data
                        .get_card_expiry_month_year_2_digit_with_delimiter("/".to_string())?,
                    securitycode: card_data.card_cvc.clone(),
                    credentialsonfile:
                        TrustpaymentsCredentialsOnFile::CardholderInitiatedTransaction.to_string(),
                }],
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method not supported for tokenization".to_string(),
            )
            .into()),
        }
    }
}

pub type TrustpaymentsTokenizationResponse = TrustpaymentsPaymentsResponse;

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::PaymentMethodToken,
            TrustpaymentsTokenizationResponse,
            hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    >
    for RouterData<
        hyperswitch_domain_models::router_flow_types::payments::PaymentMethodToken,
        hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData,
        PaymentsResponseData,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::PaymentMethodToken,
            TrustpaymentsTokenizationResponse,
            hyperswitch_domain_models::router_request_types::PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let response_data = item
            .response
            .responses
            .first()
            .ok_or(errors::ConnectorError::ResponseDeserializationFailed)?;

        let status = response_data.get_payment_status();
        let token = response_data
            .transactionreference
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TokenizationResponse { token }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustpaymentsErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}

impl TrustpaymentsErrorResponse {
    pub fn get_connector_error_type(&self) -> errors::ConnectorError {
        let error_code: TrustpaymentsErrorCode =
            serde_json::from_str(&format!("\"{}\"", self.code))
                .unwrap_or(TrustpaymentsErrorCode::Unknown(self.code.clone()));

        match error_code {
            TrustpaymentsErrorCode::InvalidCredentials
            | TrustpaymentsErrorCode::AuthenticationFailed
            | TrustpaymentsErrorCode::InvalidSiteReference
            | TrustpaymentsErrorCode::AccessDenied
            | TrustpaymentsErrorCode::InvalidUsernameOrPassword
            | TrustpaymentsErrorCode::AccountSuspended => {
                errors::ConnectorError::InvalidConnectorConfig {
                    config: "authentication",
                }
            }
            TrustpaymentsErrorCode::InvalidCardNumber
            | TrustpaymentsErrorCode::InvalidExpiryDate
            | TrustpaymentsErrorCode::InvalidSecurityCode
            | TrustpaymentsErrorCode::InvalidCardType
            | TrustpaymentsErrorCode::CardExpired
            | TrustpaymentsErrorCode::InvalidAmountValue => {
                errors::ConnectorError::InvalidDataFormat {
                    field_name: "payment_method_data",
                }
            }
            TrustpaymentsErrorCode::InsufficientFunds
            | TrustpaymentsErrorCode::CardDeclined
            | TrustpaymentsErrorCode::CardRestricted
            | TrustpaymentsErrorCode::InvalidMerchant
            | TrustpaymentsErrorCode::TransactionNotPermitted
            | TrustpaymentsErrorCode::ExceedsWithdrawalLimit
            | TrustpaymentsErrorCode::SecurityViolation
            | TrustpaymentsErrorCode::LostOrStolenCard
            | TrustpaymentsErrorCode::SuspectedFraud
            | TrustpaymentsErrorCode::ContactCardIssuer => {
                errors::ConnectorError::FailedAtConnector {
                    message: self.message.clone(),
                    code: self.code.clone(),
                }
            }
            TrustpaymentsErrorCode::GeneralProcessingError
            | TrustpaymentsErrorCode::SystemError
            | TrustpaymentsErrorCode::CommunicationError
            | TrustpaymentsErrorCode::Timeout
            | TrustpaymentsErrorCode::InvalidRequest => {
                errors::ConnectorError::ProcessingStepFailed(None)
            }
            TrustpaymentsErrorCode::Processing => errors::ConnectorError::ProcessingStepFailed(
                Some(bytes::Bytes::from("Transaction is being processed")),
            ),
            TrustpaymentsErrorCode::MissingRequiredField
            | TrustpaymentsErrorCode::InvalidFieldFormat
            | TrustpaymentsErrorCode::InvalidFieldValue
            | TrustpaymentsErrorCode::FieldTooLong
            | TrustpaymentsErrorCode::FieldTooShort
            | TrustpaymentsErrorCode::InvalidCurrency
            | TrustpaymentsErrorCode::InvalidAmount
            | TrustpaymentsErrorCode::NoSearchableFilter => {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "request_data",
                }
            }
            TrustpaymentsErrorCode::Success => errors::ConnectorError::ProcessingStepFailed(Some(
                bytes::Bytes::from("Unexpected success code in error response"),
            )),
            TrustpaymentsErrorCode::Unknown(_) => errors::ConnectorError::ProcessingStepFailed(
                Some(bytes::Bytes::from(self.message.clone())),
            ),
        }
    }
}

impl From<TrustpaymentsPaymentResponseData> for TrustpaymentsErrorResponse {
    fn from(response: TrustpaymentsPaymentResponseData) -> Self {
        let error_reason = response.get_error_reason();
        Self {
            status_code: if response.errorcode.is_success() {
                200
            } else {
                400
            },
            code: response.errorcode.to_string(),
            message: response.errormessage,
            reason: error_reason,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
        }
    }
}

impl TrustpaymentsPaymentResponseData {
    pub fn get_refund_status(&self) -> enums::RefundStatus {
        match self.errorcode {
            TrustpaymentsErrorCode::Success => match &self.settlestatus {
                Some(TrustpaymentsSettleStatus::Settled) => enums::RefundStatus::Success,
                Some(TrustpaymentsSettleStatus::PendingSettlement) => enums::RefundStatus::Pending,
                Some(TrustpaymentsSettleStatus::ManualCapture) => enums::RefundStatus::Failure,
                Some(TrustpaymentsSettleStatus::Voided) => enums::RefundStatus::Failure,
                Some(TrustpaymentsSettleStatus::PendingSettlementRedirect) => {
                    enums::RefundStatus::Pending
                }
                Some(TrustpaymentsSettleStatus::SettledRedirect) => enums::RefundStatus::Success,
                None => enums::RefundStatus::Success,
            },
            TrustpaymentsErrorCode::Processing => enums::RefundStatus::Pending,
            _ => enums::RefundStatus::Failure,
        }
    }
}
