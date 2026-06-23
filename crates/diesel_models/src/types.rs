#[cfg(feature = "v2")]
use common_enums::enums::PaymentConnectorTransmission;
#[cfg(feature = "v2")]
use common_utils::id_type;
use common_utils::{
    hashing::HashedString,
    pii,
    types::{MinorUnit, StringMajorUnit},
};
use diesel::{
    sql_types::{Json, Jsonb},
    AsExpression, FromSqlRow,
};
use hyperswitch_masking::{Secret, WithType};
use serde::{self, Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
pub struct OrderDetailsWithAmount {
    /// Name of the product that is being purchased
    pub product_name: String,
    /// The quantity of the product to be purchased
    pub quantity: u16,
    /// the amount per quantity of product
    pub amount: MinorUnit,
    // Does the order includes shipping
    pub requires_shipping: Option<bool>,
    /// The image URL of the product
    pub product_img_link: Option<String>,
    /// ID of the product that is being purchased
    pub product_id: Option<String>,
    /// Category of the product that is being purchased
    pub category: Option<String>,
    /// Sub category of the product that is being purchased
    pub sub_category: Option<String>,
    /// Brand of the product that is being purchased
    pub brand: Option<String>,
    /// Type of the product that is being purchased
    pub product_type: Option<common_enums::ProductType>,
    /// The tax code for the product
    pub product_tax_code: Option<String>,
    /// tax rate applicable to the product
    pub tax_rate: Option<f64>,
    /// total tax amount applicable to the product
    pub total_tax_amount: Option<MinorUnit>,
    /// description of the product
    pub description: Option<String>,
    /// stock keeping unit of the product
    pub sku: Option<String>,
    /// universal product code of the product
    pub upc: Option<String>,
    /// commodity code of the product
    pub commodity_code: Option<String>,
    /// unit of measure of the product
    pub unit_of_measure: Option<String>,
    /// total amount of the product
    pub total_amount: Option<MinorUnit>,
    /// discount amount on the unit
    pub unit_discount_amount: Option<MinorUnit>,
}

impl hyperswitch_masking::SerializableSecret for OrderDetailsWithAmount {}

common_utils::impl_to_sql_from_sql_json!(OrderDetailsWithAmount);

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
    /// Pix Automatico additional details for Push and QR flows
    pub pix_automatico_additional_details: Option<PixAutomaticoAdditionalDetails>,
    /// Extra information for Finix connector for fraud checks and risk evaluation
    pub finix_additional_details: Option<FinixAdditionalDetails>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct BoletoAdditionalDetails {
    /// Due Date for the Boleto
    #[serde(with = "common_utils::custom_serde::date_only_optional")]
    pub due_date: Option<time::PrimitiveDateTime>,
    // It tells the bank what type of commercial document created the boleto. Why does this boleto exist? What kind of transaction or contract caused it?
    pub document_kind: Option<common_enums::enums::BoletoDocumentKind>,
    // This field tells the bank how the boleto can be paid — whether the payer must pay the exact amount, can pay a different amount, or pay in parts.
    pub payment_type: Option<common_enums::enums::BoletoPaymentType>,
    // It is a number which shows a contract between merchant and bank
    pub covenant_code: Option<Secret<String>>,
    /// Pix identification details
    pub pix_key: Option<common_enums::enums::PixKey>,
    /// Rules for applying discounts
    pub discount_rules: Option<SantanderPaymentDiscountRules>,
    /// Rules for late payments (Interest and Fines)
    pub penalties: Option<PenaltyRules>,
    /// Legal or administrative actions for non-payment (Protest/Write-off)
    pub collection_actions: Option<CollectionActions>,
    /// Constraints on how the payment can be made (Partial payments/Limits)
    pub payment_constraints: Option<BoletoPaymentTypeConstraints>,
    /// Beneficiary details
    pub beneficiary: Option<BeneficiaryDetails>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SantanderPaymentDiscountRules {
    /// Type of discount applied to the payment.
    pub discount_type: Option<DiscountType>,
    /// Discount tiers applicable to the payment.
    pub tiers: Vec<DiscountTier>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscountType {
    /// No discount logic will be applied.
    #[default]
    Standard,
    /// A fixed amount reduction if paid on or before a specific date.
    FixedDate,
    /// A sliding discount calculated per calendar day until the due date.
    DailyCalendar,
    /// A sliding discount calculated per business day until the due date.
    DailyBusiness,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DiscountTier {
    /// The discount value.
    pub amount: Option<StringMajorUnit>,
    /// The ISO-8601 date until which this discount is valid.
    #[serde(default, with = "common_utils::custom_serde::date_only_optional")]
    pub end_date: Option<time::PrimitiveDateTime>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PenaltyRules {
    /// Fixed fee applied once after the due date.
    pub fixed_penalty: Option<PenaltyDetail>,
    /// Recurring cost applied over time.
    pub interest: Option<InterestDetail>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InterestDetail {
    /// Percentage of Juros (Interest).
    pub interest_percentage: Option<StringMajorUnit>,
    /// Percentage of IOF (Financial Operations Tax).
    pub iof_percentage: Option<StringMajorUnit>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PenaltyDetail {
    /// The numeric value as a string to preserve decimal precision.
    pub value: Option<StringMajorUnit>,
    /// Days after due date before this applies.
    pub grace_period_days: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CollectionActions {
    /// Logic for legal protest.
    pub legal_protest: Option<ProtestRules>,
    /// Days after which the bill is automatically cancelled/written off.
    pub auto_write_off_days: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ProtestRules {
    /// The timing logic for when the protest should occur.
    pub protest_type: Option<ProtestType>,
    /// Number of days after the due date to initiate the protest.
    pub days_after_due_date: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtestType {
    /// No legal protest will be initiated.
    Disabled,
    /// Count is based on calendar days.
    CalendarDays,
    /// Count is based on business days.
    BusinessDays,
    /// Protest logic is handled based on the merchant's bank agreement.
    ContractDefault,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "details")]
pub enum BoletoPaymentTypeConstraints {
    /// Only the exact nominal amount can be paid.
    FixedAmount,
    /// The payer may pay any amount within an allowed range.
    FlexibleAmount(FlexibleAmountDetails),
    /// The payer may make multiple payments, up to a specific limit.
    Installment(InstallmentDetails),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FlexibleAmountDetails {
    /// Minimum value allowed.
    pub min_value: Option<StringMajorUnit>,
    /// Maximum value allowed.
    pub max_value: Option<StringMajorUnit>,
    /// Defines if the min/max values are percentages or flat amounts.
    pub value_type: Option<CalculationType>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InstallmentDetails {
    /// Maximum number of partial payments allowed.
    pub max_partial_payments: Option<u32>,
    /// Defines if the values are percentages or flat amounts.
    pub value_type: Option<CalculationType>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CalculationType {
    /// The value is treated as a percentage.
    Percentage,
    /// The value is treated as a fixed monetary amount.
    FlatAmount,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BeneficiaryDetails {
    /// The full legal name of the individual or entity receiving the funds.
    pub name: Option<String>,
    /// The customer's unique identification number.
    pub document_number: Option<String>,
    /// The category of identification provided.
    pub document_type: Option<common_types::customers::DocumentKind>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct FinixAdditionalDetails {
    /// The fraud session ID used for Finix fraud detection
    pub fraud_session_id: Option<String>,
}

#[cfg(feature = "v2")]
impl FeatureMetadata {
    pub fn get_payment_method_sub_type(&self) -> Option<common_enums::PaymentMethodType> {
        self.payment_revenue_recovery_metadata
            .as_ref()
            .map(|rrm| rrm.payment_method_subtype)
    }

    pub fn get_payment_method_type(&self) -> Option<common_enums::PaymentMethod> {
        self.payment_revenue_recovery_metadata
            .as_ref()
            .map(|recovery_metadata| recovery_metadata.payment_method_type)
    }

    pub fn get_billing_merchant_connector_account_id(
        &self,
    ) -> Option<id_type::MerchantConnectorAccountId> {
        self.payment_revenue_recovery_metadata
            .as_ref()
            .map(|recovery_metadata| recovery_metadata.billing_connector_id.clone())
    }

    /// Compare with request payload feature metadata and return the one from request if different
    /// Returns None if not present in request payload
    pub fn compare_with_request_payload(
        &self,
        request_feature_metadata: Option<&FeatureMetadata>,
    ) -> Option<FeatureMetadata> {
        request_feature_metadata.and_then(|req_metadata| {
            // If they differ, return the request version (indicating updates from original request)
            // If they match, return None (indicating they're the same)
            if req_metadata != self {
                Some(req_metadata.clone())
            } else {
                None
            }
        })
    }

    // TODO: Check search_tags for relevant payment method type
    // TODO: Check redirect_response metadata if applicable
    // TODO: Check apple_pay_recurring_details metadata if applicable
}

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
    /// Pix Automatico additional details for Push and QR flows
    pub pix_automatico_additional_details: Option<PixAutomaticoAdditionalDetails>,
    /// Extra information for Finix connector for fraud checks and risk evaluation
    pub finix_additional_details: Option<FinixAdditionalDetails>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub enum PixAdditionalDetails {
    #[serde(rename = "immediate")]
    Immediate(ImmediateExpirationTime),
    #[serde(rename = "scheduled")]
    Scheduled(ScheduledExpirationTime),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct ImmediateExpirationTime {
    /// Expiration time in seconds
    pub time: u32,
    /// Pix identification details
    pub pix_key: Option<common_enums::enums::PixKey>,
}

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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PixAutomaticoAdditionalDetails {
    PixAutomaticoPush(PixAutomaticoPushData),
    PixAutomaticoQr(PixAutomaticoQrData),
    PixAutomaticoMit(PixAutomaticoMitData),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct PixAutomaticoPushData {
    pub time: u32,
    pub retry_policy: Option<bool>,
    pub mandate_details: Option<SantanderMandateDetails>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct PixAutomaticoQrData {
    pub retry_policy: Option<bool>,
    pub mandate_details: Option<SantanderMandateDetails>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct PixAutomaticoMitData {
    pub receiver_details: Option<SantanderPixAutomaticoReceiverDetails>,
    #[serde(default, with = "common_utils::custom_serde::date_only_optional")]
    pub mandate_execution_date: Option<time::PrimitiveDateTime>,
    pub auto_adjust_date: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct SantanderMandateDetails {
    pub fixed_recurring_amount: Option<MinorUnit>,
    pub min_recurring_amount: Option<MinorUnit>,
    #[serde(default, with = "common_utils::custom_serde::date_only_optional")]
    pub start_date: Option<time::PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::date_only_optional")]
    pub end_date: Option<time::PrimitiveDateTime>,
    pub periodicity: Option<SantanderMandatePeriodicity>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
#[serde(rename_all = "snake_case")]
pub enum SantanderMandatePeriodicity {
    Weekly,
    #[default]
    Monthly,
    Quarterly,
    Semiannually,
    Annually,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Current,
    Savings,
    Payment,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct SantanderPixAutomaticoReceiverDetails {
    pub branch_code: Option<Secret<String>>,
    pub account_number: Option<Secret<String>>,
    pub account_type: Option<AccountType>,
}

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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
#[serde(rename_all = "snake_case")]
pub enum RecurringPaymentIntervalUnit {
    Year,
    Month,
    Day,
    Hour,
    Minute,
}

common_utils::impl_to_sql_from_sql_json!(ApplePayRecurringDetails);
common_utils::impl_to_sql_from_sql_json!(ApplePayRegularBillingDetails);
common_utils::impl_to_sql_from_sql_json!(RecurringPaymentIntervalUnit);

common_utils::impl_to_sql_from_sql_json!(FeatureMetadata);

#[derive(Default, Debug, Eq, PartialEq, Deserialize, Serialize, Clone)]
pub struct RedirectResponse {
    pub param: Option<Secret<String>>,
    pub json_payload: Option<pii::SecretSerdeValue>,
}
impl hyperswitch_masking::SerializableSecret for RedirectResponse {}
common_utils::impl_to_sql_from_sql_json!(RedirectResponse);

#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PaymentRevenueRecoveryMetadata {
    /// Total number of billing connector + recovery retries for a payment intent.
    pub total_retry_count: u16,
    /// Flag for the payment connector's call
    pub payment_connector_transmission: PaymentConnectorTransmission,
    /// Billing Connector Id to update the invoices
    pub billing_connector_id: id_type::MerchantConnectorAccountId,
    /// Payment Connector Id to retry the payments
    pub active_attempt_payment_connector_id: id_type::MerchantConnectorAccountId,
    /// Billing Connector Payment Details
    pub billing_connector_payment_details: BillingConnectorPaymentDetails,
    ///Payment Method Type
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg(feature = "v2")]
pub struct BillingConnectorPaymentDetails {
    /// Payment Processor Token to process the Revenue Recovery Payment
    pub payment_processor_token: String,
    /// Billing Connector's Customer Id
    pub connector_customer_id: String,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum BillingConnectorPaymentMethodDetails {
    Card(BillingConnectorAdditionalCardInfo),
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BillingConnectorAdditionalCardInfo {
    /// Card Network
    pub card_network: Option<common_enums::enums::CardNetwork>,
    /// Card Issuer
    pub card_issuer: Option<String>,
}
