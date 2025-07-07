//! Payment related types

use std::collections::HashMap;

use common_enums::enums;
#[cfg(feature = "v2")]
use common_utils::hashing::HashedString;
use common_utils::{date_time, errors, events, impl_to_sql_from_sql_json, pii, types::MinorUnit};
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use euclid::frontend::{
    ast::Program,
    dir::{DirKeyKind, EuclidDirFilter},
};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::domain::{AdyenSplitData, XenditSplitSubMerchantData};
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
/// Fee information for Split Payments to be charged on the payment being collected
pub enum SplitPaymentsRequest {
    /// StripeSplitPayment
    StripeSplitPayment(StripeSplitPaymentRequest),
    /// AdyenSplitPayment
    AdyenSplitPayment(AdyenSplitData),
    /// XenditSplitPayment
    XenditSplitPayment(XenditSplitRequest),
}
impl_to_sql_from_sql_json!(SplitPaymentsRequest);

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Fee information for Split Payments to be charged on the payment being collected for Stripe
pub struct StripeSplitPaymentRequest {
    /// Stripe's charge type
    #[schema(value_type = PaymentChargeType, example = "direct")]
    pub charge_type: enums::PaymentChargeType,

    /// Platform fees to be collected on the payment
    #[schema(value_type = i64, example = 6540)]
    pub application_fees: Option<MinorUnit>,

    /// Identifier for the reseller's account where the funds were transferred
    pub transfer_account_id: String,
}
impl_to_sql_from_sql_json!(StripeSplitPaymentRequest);

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
/// Hashmap to store mca_id's with product names
pub struct AuthenticationConnectorAccountMap(
    HashMap<enums::AuthenticationProduct, common_utils::id_type::MerchantConnectorAccountId>,
);
impl_to_sql_from_sql_json!(AuthenticationConnectorAccountMap);

impl AuthenticationConnectorAccountMap {
    /// fn to get click to pay connector_account_id
    pub fn get_click_to_pay_connector_account_id(
        &self,
    ) -> Result<common_utils::id_type::MerchantConnectorAccountId, errors::ValidationError> {
        self.0
            .get(&enums::AuthenticationProduct::ClickToPay)
            .ok_or(errors::ValidationError::MissingRequiredField {
                field_name: "authentication_product_id.click_to_pay".to_string(),
            })
            .cloned()
    }
}

#[derive(
    Serialize, Default, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
/// ConditionalConfigs
pub struct ConditionalConfigs {
    /// Override 3DS
    pub override_3ds: Option<common_enums::AuthenticationType>,
}
impl EuclidDirFilter for ConditionalConfigs {
    const ALLOWED: &'static [DirKeyKind] = &[
        DirKeyKind::PaymentMethod,
        DirKeyKind::CardType,
        DirKeyKind::CardNetwork,
        DirKeyKind::MetaData,
        DirKeyKind::PaymentAmount,
        DirKeyKind::PaymentCurrency,
        DirKeyKind::CaptureMethod,
        DirKeyKind::BillingCountry,
        DirKeyKind::BusinessCountry,
    ];
}

impl_to_sql_from_sql_json!(ConditionalConfigs);

/// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
#[derive(
    Default,
    Eq,
    PartialEq,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    Clone,
    AsExpression,
    ToSchema,
)]
#[serde(deny_unknown_fields)]
#[diesel(sql_type = Jsonb)]
pub struct CustomerAcceptance {
    /// Type of acceptance provided by the
    #[schema(example = "online")]
    pub acceptance_type: AcceptanceType,
    /// Specifying when the customer acceptance was provided
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub accepted_at: Option<PrimitiveDateTime>,
    /// Information required for online mandate generation
    pub online: Option<OnlineMandate>,
}

impl_to_sql_from_sql_json!(CustomerAcceptance);

impl CustomerAcceptance {
    /// Get the IP address
    pub fn get_ip_address(&self) -> Option<String> {
        self.online
            .as_ref()
            .and_then(|data| data.ip_address.as_ref().map(|ip| ip.peek().to_owned()))
    }

    /// Get the User Agent
    pub fn get_user_agent(&self) -> Option<String> {
        self.online.as_ref().map(|data| data.user_agent.clone())
    }

    /// Get when the customer acceptance was provided
    pub fn get_accepted_at(&self) -> PrimitiveDateTime {
        self.accepted_at.unwrap_or_else(date_time::now)
    }
}

impl masking::SerializableSecret for CustomerAcceptance {}

#[derive(
    Default, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone, Copy, ToSchema,
)]
#[serde(rename_all = "lowercase")]
/// This is used to indicate if the mandate was accepted online or offline
pub enum AcceptanceType {
    /// Online
    Online,
    /// Offline
    #[default]
    Offline,
}

#[derive(
    Default,
    Eq,
    PartialEq,
    Debug,
    serde::Deserialize,
    serde::Serialize,
    AsExpression,
    Clone,
    ToSchema,
)]
#[serde(deny_unknown_fields)]
/// Details of online mandate
#[diesel(sql_type = Jsonb)]
pub struct OnlineMandate {
    /// Ip address of the customer machine from which the mandate was created
    #[schema(value_type = String, example = "123.32.25.123")]
    pub ip_address: Option<Secret<String, pii::IpAddress>>,
    /// The user-agent of the customer's browser
    pub user_agent: String,
}

impl_to_sql_from_sql_json!(OnlineMandate);

#[derive(Serialize, Deserialize, Debug, Clone, FromSqlRow, AsExpression, ToSchema)]
#[diesel(sql_type = Jsonb)]
/// DecisionManagerRecord
pub struct DecisionManagerRecord {
    /// Name of the Decision Manager
    pub name: String,
    /// Program to be executed
    pub program: Program<ConditionalConfigs>,
    /// Created at timestamp
    pub created_at: i64,
}

impl events::ApiEventMetric for DecisionManagerRecord {
    fn get_api_event_type(&self) -> Option<events::ApiEventsType> {
        Some(events::ApiEventsType::Routing)
    }
}
impl_to_sql_from_sql_json!(DecisionManagerRecord);

/// DecisionManagerResponse
pub type DecisionManagerResponse = DecisionManagerRecord;

/// Fee information to be charged on the payment being collected via Stripe
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
pub struct StripeChargeResponseData {
    /// Identifier for charge created for the payment
    pub charge_id: Option<String>,

    /// Type of charge (connector specific)
    #[schema(value_type = PaymentChargeType, example = "direct")]
    pub charge_type: enums::PaymentChargeType,

    /// Platform fees collected on the payment
    #[schema(value_type = i64, example = 6540)]
    pub application_fees: Option<MinorUnit>,

    /// Identifier for the reseller's account where the funds were transferred
    pub transfer_account_id: String,
}
impl_to_sql_from_sql_json!(StripeChargeResponseData);

/// Charge Information
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum ConnectorChargeResponseData {
    /// StripeChargeResponseData
    StripeSplitPayment(StripeChargeResponseData),
    /// AdyenChargeResponseData
    AdyenSplitPayment(AdyenSplitData),
    /// XenditChargeResponseData
    XenditSplitPayment(XenditChargeResponseData),
}

impl_to_sql_from_sql_json!(ConnectorChargeResponseData);

/// Fee information to be charged on the payment being collected via xendit
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
pub struct XenditSplitRoute {
    /// Amount of payments to be split
    pub flat_amount: Option<MinorUnit>,
    /// Amount of payments to be split, using a percent rate as unit
    pub percent_amount: Option<i64>,
    /// Currency code
    #[schema(value_type = Currency, example = "USD")]
    pub currency: enums::Currency,
    ///  ID of the destination account where the amount will be routed to
    pub destination_account_id: String,
    /// Reference ID which acts as an identifier of the route itself
    pub reference_id: String,
}
impl_to_sql_from_sql_json!(XenditSplitRoute);

/// Fee information to be charged on the payment being collected via xendit
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
pub struct XenditMultipleSplitRequest {
    /// Name to identify split rule. Not required to be unique. Typically based on transaction and/or sub-merchant types.
    pub name: String,
    /// Description to identify fee rule
    pub description: String,
    /// The sub-account user-id that you want to make this transaction for.
    pub for_user_id: Option<String>,
    /// Array of objects that define how the platform wants to route the fees and to which accounts.
    pub routes: Vec<XenditSplitRoute>,
}
impl_to_sql_from_sql_json!(XenditMultipleSplitRequest);

/// Xendit Charge Request
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum XenditSplitRequest {
    /// Split Between Multiple Accounts
    MultipleSplits(XenditMultipleSplitRequest),
    /// Collect Fee for Single Account
    SingleSplit(XenditSplitSubMerchantData),
}

impl_to_sql_from_sql_json!(XenditSplitRequest);

/// Charge Information
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum XenditChargeResponseData {
    /// Split Between Multiple Accounts
    MultipleSplits(XenditMultipleSplitResponse),
    /// Collect Fee for Single Account
    SingleSplit(XenditSplitSubMerchantData),
}

impl_to_sql_from_sql_json!(XenditChargeResponseData);

/// Fee information charged on the payment being collected via xendit
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
#[serde(deny_unknown_fields)]
pub struct XenditMultipleSplitResponse {
    /// Identifier for split rule created for the payment
    pub split_rule_id: String,
    /// The sub-account user-id that you want to make this transaction for.
    pub for_user_id: Option<String>,
    /// Name to identify split rule. Not required to be unique. Typically based on transaction and/or sub-merchant types.
    pub name: String,
    /// Description to identify fee rule
    pub description: String,
    /// Array of objects that define how the platform wants to route the fees and to which accounts.
    pub routes: Vec<XenditSplitRoute>,
}
impl_to_sql_from_sql_json!(XenditMultipleSplitResponse);



#[allow(missing_docs)]
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, diesel::AsExpression)]
#[diesel(sql_type = Jsonb)]
pub struct TaxDetails {
    /// This is the tax related information that is calculated irrespective of any payment method.
    /// This is calculated when the order is created with shipping details
    pub default: Option<DefaultTax>,

    /// This is the tax related information that is calculated based on the payment method
    /// This is calculated when calling the /calculate_tax API
    pub payment_method_type: Option<PaymentMethodTypeTax>,
}

impl TaxDetails {
    /// Get the tax amount
    /// If default tax is present, return the default tax amount
    /// If default tax is not present, return the tax amount based on the payment method if it matches the provided payment method type
    pub fn get_tax_amount(&self, payment_method: Option<enums::PaymentMethodType>) -> Option<MinorUnit> {
        self.payment_method_type
            .as_ref()
            .zip(payment_method)
            .filter(|(payment_method_type_tax, payment_method)| {
                payment_method_type_tax.pmt == *payment_method
            })
            .map(|(payment_method_type_tax, _)| payment_method_type_tax.order_tax_amount)
            .or_else(|| self.get_default_tax_amount())
    }

    /// Get the default tax amount
    pub fn get_default_tax_amount(&self) -> Option<MinorUnit> {
        self.default
            .as_ref()
            .map(|default_tax_details| default_tax_details.order_tax_amount)
    }
}

common_utils::impl_to_sql_from_sql_json!(TaxDetails);

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaymentMethodTypeTax {
    pub order_tax_amount: MinorUnit,
    pub pmt: enums::PaymentMethodType,
}

#[allow(missing_docs)]

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct DefaultTax {
    pub order_tax_amount: MinorUnit,
}




#[allow(missing_docs)]
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
}

impl masking::SerializableSecret for OrderDetailsWithAmount {}

common_utils::impl_to_sql_from_sql_json!(OrderDetailsWithAmount);


#[allow(missing_docs)]
#[cfg(feature = "v2")]
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct FeatureMetadata {
    /// Redirection response coming in request as metadata field only for redirection scenarios
    pub redirect_response: Option<RedirectResponse>,
    /// Additional tags to be used for global search
    pub search_tags: Option<Vec<HashedString<masking::WithType>>>,
    /// Recurring payment details required for apple pay Merchant Token
    pub apple_pay_recurring_details: Option<ApplePayRecurringDetails>,
    /// revenue recovery data for payment intent
    pub payment_revenue_recovery_metadata: Option<PaymentRevenueRecoveryMetadata>,
}

common_utils::impl_to_sql_from_sql_json!(FeatureMetadata);

#[allow(missing_docs)]
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
    ) -> Option<common_utils::id_type::MerchantConnectorAccountId> {
        self.payment_revenue_recovery_metadata
            .as_ref()
            .map(|recovery_metadata| recovery_metadata.billing_connector_id.clone())
    }

    // TODO: Check search_tags for relevant payment method type
    // TODO: Check redirect_response metadata if applicable
    // TODO: Check apple_pay_recurring_details metadata if applicable
}

#[allow(missing_docs)]
#[derive(Default, Debug, Eq, PartialEq, Deserialize, Serialize, Clone)]
pub struct RedirectResponse {
    pub param: Option<Secret<String>>,
    pub json_payload: Option<pii::SecretSerdeValue>,
}
impl masking::SerializableSecret for RedirectResponse {}
common_utils::impl_to_sql_from_sql_json!(RedirectResponse);

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Json)]
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

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct ApplePayRegularBillingDetails {
    /// The label that Apple Pay displays to the user in the payment sheet with the recurring details
    pub label: String,
    /// The date of the first payment
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recurring_payment_start_date: Option<PrimitiveDateTime>,
    /// The date of the final payment
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recurring_payment_end_date: Option<PrimitiveDateTime>,
    /// The amount of time — in calendar units, such as day, month, or year — that represents a fraction of the total payment interval
    pub recurring_payment_interval_unit: Option<RecurringPaymentIntervalUnit>,
    /// The number of interval units that make up the total payment interval
    pub recurring_payment_interval_count: Option<i32>,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Json)]
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


#[allow(missing_docs)]
#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PaymentRevenueRecoveryMetadata {
    /// Total number of billing connector + recovery retries for a payment intent.
    pub total_retry_count: u16,
    /// Flag for the payment connector's call
    pub payment_connector_transmission: enums::PaymentConnectorTransmission,
    /// Billing Connector Id to update the invoices
    pub billing_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    /// Payment Connector Id to retry the payments
    pub active_attempt_payment_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    /// Billing Connector Payment Details
    pub billing_connector_payment_details: BillingConnectorPaymentDetails,
    ///Payment Method Type
    pub payment_method_type: enums::PaymentMethod,
    /// PaymentMethod Subtype
    pub payment_method_subtype: enums::PaymentMethodType,
    /// The name of the payment connector through which the payment attempt was made.
    pub connector: common_enums::connector_enums::Connector,
    /// Time at which next invoice will be created
    pub invoice_next_billing_time: Option<PrimitiveDateTime>,
    /// Extra Payment Method Details that are needed to be stored
    pub billing_connector_payment_method_details: Option<BillingConnectorPaymentMethodDetails>,
    /// First Payment Attempt Payment Gateway Error Code
    pub first_payment_attempt_pg_error_code: Option<String>,
    /// First Payment Attempt Network Error Code
    pub first_payment_attempt_network_decline_code: Option<String>,
    /// First Payment Attempt Network Advice Code
    pub first_payment_attempt_network_advice_code: Option<String>,
}
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg(feature = "v2")]
pub struct BillingConnectorPaymentDetails {
    /// Payment Processor Token to process the Revenue Recovery Payment
    pub payment_processor_token: String,
    /// Billing Connector's Customer Id
    pub connector_customer_id: String,
}

#[allow(missing_docs)]
#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum BillingConnectorPaymentMethodDetails {
    Card(BillingConnectorAdditionalCardInfo),
}

#[allow(missing_docs)]
#[cfg(feature = "v2")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BillingConnectorAdditionalCardInfo {
    /// Card Network
    pub card_network: Option<enums::CardNetwork>,
    /// Card Issuer
    pub card_issuer: Option<String>,
}


#[allow(missing_docs)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression, PartialEq)]
#[diesel(sql_type = Jsonb)]
pub struct PaymentLinkConfigRequestForPayments {
    /// custom theme for the payment link
    pub theme: Option<String>,
    /// merchant display logo
    pub logo: Option<String>,
    /// Custom merchant name for payment link
    pub seller_name: Option<String>,
    /// Custom layout for sdk
    pub sdk_layout: Option<String>,
    /// Display only the sdk for payment link
    pub display_sdk_only: Option<bool>,
    /// Enable saved payment method option for payment link
    pub enabled_saved_payment_method: Option<bool>,
    /// Hide card nickname field option for payment link
    pub hide_card_nickname_field: Option<bool>,
    /// Show card form by default for payment link
    pub show_card_form_by_default: Option<bool>,
    /// Dynamic details related to merchant to be rendered in payment link
    pub transaction_details: Option<Vec<PaymentLinkTransactionDetails>>,
    /// Configurations for the background image for details section
    pub background_image: Option<PaymentLinkBackgroundImageConfig>,
    /// Custom layout for details section
    pub details_layout: Option<common_enums::PaymentLinkDetailsLayout>,
    /// Text for payment link's handle confirm button
    pub payment_button_text: Option<String>,
    /// Skip the status screen after payment completion
    pub skip_status_screen: Option<bool>,
    /// Text for customizing message for card terms
    pub custom_message_for_card_terms: Option<String>,
    /// Custom background colour for payment link's handle confirm button
    pub payment_button_colour: Option<String>,
    /// Custom text colour for payment link's handle confirm button
    pub payment_button_text_colour: Option<String>,
    /// Custom background colour for the payment link
    pub background_colour: Option<String>,
    /// SDK configuration rules
    pub sdk_ui_rules:
        Option<HashMap<String, HashMap<String, String>>>,
    /// Payment link configuration rules
    pub payment_link_ui_rules:
        Option<HashMap<String, HashMap<String, String>>>,
    /// Flag to enable the button only when the payment form is ready for submission
    pub enable_button_only_on_form_ready: Option<bool>,
    /// Optional header for the SDK's payment form
    pub payment_form_header_text: Option<String>,
    /// Label type in the SDK's payment form
    pub payment_form_label_type: Option<common_enums::PaymentLinkSdkLabelType>,
    /// Boolean for controlling whether or not to show the explicit consent for storing cards
    pub show_card_terms: Option<common_enums::PaymentLinkShowSdkTerms>,
    /// Boolean to control payment button text for setup mandate calls
    pub is_setup_mandate_flow: Option<bool>,
    /// Hex color for the CVC icon during error state
    pub color_icon_card_cvc_error: Option<String>,
}

common_utils::impl_to_sql_from_sql_json!(PaymentLinkConfigRequestForPayments);

#[allow(missing_docs)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct PaymentLinkTransactionDetails {
    /// Key for the transaction details
    pub key: String,
    /// Value for the transaction details
    pub value: String,
    /// UI configuration for the transaction details
    pub ui_configuration: Option<TransactionDetailsUiConfiguration>,
}

common_utils::impl_to_sql_from_sql_json!(PaymentLinkTransactionDetails);

#[allow(missing_docs)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct TransactionDetailsUiConfiguration {
    /// Position of the key-value pair in the UI
    pub position: Option<i8>,
    /// Whether the key should be bold
    pub is_key_bold: Option<bool>,
    /// Whether the value should be bold
    pub is_value_bold: Option<bool>,
}

common_utils::impl_to_sql_from_sql_json!(TransactionDetailsUiConfiguration);


#[allow(missing_docs)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct PaymentLinkBackgroundImageConfig {
    pub url: common_utils::types::Url,
    pub position: Option<common_enums::ElementPosition>,
    pub size: Option<common_enums::ElementSize>,
}



#[allow(missing_docs)]
#[cfg(feature = "v2")]
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, diesel::AsExpression,
)]
#[diesel(sql_type = Jsonb)]
pub struct ConnectorTokenDetails {
    pub connector_mandate_id: Option<String>,
    pub connector_token_request_reference_id: Option<String>,
}

#[cfg(feature = "v2")]
common_utils::impl_to_sql_from_sql_json!(ConnectorTokenDetails);

#[allow(missing_docs)]
#[cfg(feature = "v2")]
impl ConnectorTokenDetails {
    pub fn get_connector_token_request_reference_id(&self) -> Option<String> {
        self.connector_token_request_reference_id.clone()
    }
}