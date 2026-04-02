//! Storage types shared across request/response and database types.
//! These are types that are serialized as JSON/JSONB in the database.

use common_utils::{hashing::HashedString, pii};
use diesel::{sql_types::Json, AsExpression, FromSqlRow};
use hyperswitch_masking::{Secret, WithType};
use serde::{Deserialize, Serialize};

// --- FeatureMetadata ---

/// Feature metadata for v2
#[cfg(feature = "v2")]
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct FeatureMetadata {
    /// Redirection response coming in request as metadata field only for redirection scenarios
    pub redirect_response: Option<RedirectResponse>,
    /// Additional tags to be used for global search
    pub search_tags: Option<Vec<HashedString<WithType>>>,
    /// Recurring payment details required for apple pay Merchant Token
    pub apple_pay_recurring_details: Option<ApplePayRecurringDetails>,
    /// revenue recovery data for payment intent
    pub payment_revenue_recovery_metadata: Option<PaymentRevenueRecoveryMetadata>,
    /// Additional information related to pix like expiry time etc for QR Code payments
    pub pix_additional_details: Option<PixAdditionalDetails>,
    /// Extra information like fine percentage, interest percentage etc required for Pix payment method
    pub boleto_additional_details: Option<BoletoAdditionalDetails>,
}

/// Feature metadata for v1
#[cfg(feature = "v1")]
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct FeatureMetadata {
    /// Redirection response coming in request as metadata field only for redirection scenarios
    pub redirect_response: Option<RedirectResponse>,
    /// Additional tags to be used for global search
    pub search_tags: Option<Vec<HashedString<WithType>>>,
    /// Recurring payment details required for apple pay Merchant Token
    pub apple_pay_recurring_details: Option<ApplePayRecurringDetails>,
    /// The system that the gateway is integrated with, e.g., `Direct`(through hyperswitch), `UnifiedConnectorService`(through ucs), etc.
    pub gateway_system: Option<common_enums::GatewaySystem>,
    /// Additional information related to pix like expiry time etc for QR Code payments
    pub pix_additional_details: Option<PixAdditionalDetails>,
    /// Extra information like fine percentage, interest percentage etc required for Pix payment method
    pub boleto_additional_details: Option<BoletoAdditionalDetails>,
}

#[cfg(feature = "v2")]
impl FeatureMetadata {
    /// Get payment method sub type from revenue recovery metadata
    pub fn get_payment_method_sub_type(&self) -> Option<common_enums::PaymentMethodType> {
        self.payment_revenue_recovery_metadata
            .as_ref()
            .map(|rrm| rrm.payment_method_subtype)
    }

    /// Get payment method type from revenue recovery metadata
    pub fn get_payment_method_type(&self) -> Option<common_enums::PaymentMethod> {
        self.payment_revenue_recovery_metadata
            .as_ref()
            .map(|recovery_metadata| recovery_metadata.payment_method_type)
    }

    /// Get billing merchant connector account ID from revenue recovery metadata
    pub fn get_billing_merchant_connector_account_id(
        &self,
    ) -> Option<common_utils::id_type::MerchantConnectorAccountId> {
        self.payment_revenue_recovery_metadata
            .as_ref()
            .map(|recovery_metadata| recovery_metadata.billing_connector_id.clone())
    }
}

common_utils::impl_to_sql_from_sql_json!(FeatureMetadata);

// --- Pix Additional Details ---

/// Additional information related to pix
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub enum PixAdditionalDetails {
    /// Immediate expiration
    #[serde(rename = "immediate")]
    Immediate(ImmediateExpirationTime),
    /// Scheduled expiration
    #[serde(rename = "scheduled")]
    Scheduled(ScheduledExpirationTime),
}

/// Immediate expiration time for Pix
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct ImmediateExpirationTime {
    /// Expiration time in seconds
    pub time: u32,
    /// Pix identification details
    pub pix_key: Option<common_enums::enums::PixKey>,
}

/// Scheduled expiration time for Pix
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct ScheduledExpirationTime {
    /// Expiration time in terms of date, format: YYYY-MM-DD
    #[serde(with = "common_utils::custom_serde::date_only")]
    pub date: time::PrimitiveDateTime,
    /// Days after expiration date for which the QR code remains valid
    pub validity_after_expiration: Option<u32>,
    /// Pix identification details
    pub pix_key: Option<common_enums::enums::PixKey>,
}

// --- Boleto Additional Details ---

/// Extra information for Boleto payment method
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct BoletoAdditionalDetails {
    /// Due Date for the Boleto
    #[serde(with = "common_utils::custom_serde::date_only_optional")]
    pub due_date: Option<time::PrimitiveDateTime>,
    // It tells the bank what type of commercial document created the boleto.
    /// Document kind
    pub document_kind: Option<common_enums::enums::BoletoDocumentKind>,
    // This field tells the bank how the boleto can be paid.
    /// Payment type
    pub payment_type: Option<common_enums::enums::BoletoPaymentType>,
    // It is a number which shows a contract between merchant and bank
    /// Covenant code
    pub covenant_code: Option<Secret<String>>,
    /// Pix identification details
    pub pix_key: Option<common_enums::enums::PixKey>,
}

// --- Apple Pay Recurring Details ---

/// Recurring payment details for Apple Pay
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct ApplePayRecurringDetails {
    /// A description of the recurring payment that Apple Pay displays to the user in the payment sheet
    pub payment_description: String,
    /// The regular billing cycle for the recurring payment, including start and end dates, an interval, and an interval count
    pub regular_billing: ApplePayRegularBillingDetails,
    /// A localized billing agreement that the payment sheet displays to the user before the user authorizes the payment
    pub billing_agreement: Option<String>,
    /// A URL to a web page where the user can update or delete the payment method for the recurring payment
    pub management_url: common_utils::types::Url,
}

/// Regular billing details for Apple Pay recurring payments
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct ApplePayRegularBillingDetails {
    /// The label that Apple Pay displays to the user in the payment sheet with the recurring details
    pub label: String,
    /// The date of the first payment
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recurring_payment_start_date: Option<time::PrimitiveDateTime>,
    /// The date of the final payment
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recurring_payment_end_date: Option<time::PrimitiveDateTime>,
    /// The amount of time — in calendar units, such as day, month, or year — that represents a fraction of the total payment interval
    pub recurring_payment_interval_unit: Option<RecurringPaymentIntervalUnit>,
    /// The number of interval units that make up the total payment interval
    pub recurring_payment_interval_count: Option<i32>,
}

/// Unit for recurring payment intervals
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
#[serde(rename_all = "snake_case")]
pub enum RecurringPaymentIntervalUnit {
    /// Year
    Year,
    /// Month
    Month,
    /// Day
    Day,
    /// Hour
    Hour,
    /// Minute
    Minute,
}

common_utils::impl_to_sql_from_sql_json!(ApplePayRecurringDetails);
common_utils::impl_to_sql_from_sql_json!(ApplePayRegularBillingDetails);
common_utils::impl_to_sql_from_sql_json!(RecurringPaymentIntervalUnit);

// --- Redirect Response ---

/// Redirect response from payment processing
#[derive(Default, Debug, Eq, PartialEq, Deserialize, Serialize, Clone)]
pub struct RedirectResponse {
    /// Redirect parameter
    pub param: Option<Secret<String>>,
    /// JSON payload
    pub json_payload: Option<pii::SecretSerdeValue>,
}
impl hyperswitch_masking::SerializableSecret for RedirectResponse {}
common_utils::impl_to_sql_from_sql_json!(RedirectResponse);

// --- v2 Revenue Recovery Types ---

/// Payment revenue recovery metadata
#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PaymentRevenueRecoveryMetadata {
    /// Total number of billing connector + recovery retries for a payment intent.
    pub total_retry_count: u16,
    /// Flag for the payment connector's call
    pub payment_connector_transmission: common_enums::enums::PaymentConnectorTransmission,
    /// Billing Connector Id to update the invoices
    pub billing_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    /// Payment Connector Id to retry the payments
    pub active_attempt_payment_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    /// Billing Connector Payment Details
    pub billing_connector_payment_details: BillingConnectorPaymentDetails,
    /// Payment Method Type
    pub payment_method_type: common_enums::enums::PaymentMethod,
    /// PaymentMethod Subtype
    pub payment_method_subtype: common_enums::enums::PaymentMethodType,
    /// The name of the payment connector through which the payment attempt was made.
    pub connector: common_enums::connector_enums::Connector,
    /// Time at which next invoice will be created
    pub invoice_next_billing_time: Option<time::PrimitiveDateTime>,
    /// Time at which invoice started
    pub invoice_billing_started_at_time: Option<time::PrimitiveDateTime>,
    /// Extra Payment Method Details that are needed to be stored
    pub billing_connector_payment_method_details: Option<BillingConnectorPaymentMethodDetails>,
    /// First Payment Attempt Payment Gateway Error Code
    pub first_payment_attempt_pg_error_code: Option<String>,
    /// First Payment Attempt Network Error Code
    pub first_payment_attempt_network_decline_code: Option<String>,
    /// First Payment Attempt Network Advice Code
    pub first_payment_attempt_network_advice_code: Option<String>,
}

/// Billing connector payment details
#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BillingConnectorPaymentDetails {
    /// Payment Processor Token to process the Revenue Recovery Payment
    pub payment_processor_token: String,
    /// Billing Connector's Customer Id
    pub connector_customer_id: String,
}

/// Billing connector payment method details
#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum BillingConnectorPaymentMethodDetails {
    /// Card details
    Card(BillingConnectorAdditionalCardInfo),
}

/// Additional card info for billing connector
#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BillingConnectorAdditionalCardInfo {
    /// Card Network
    pub card_network: Option<common_enums::enums::CardNetwork>,
    /// Card Issuer
    pub card_issuer: Option<String>,
}
