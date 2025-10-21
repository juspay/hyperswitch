use common_enums::connector_enums::InvoiceStatus;
use common_types::payments::CustomerAcceptance;
use common_utils::{
    events::ApiEventMetric,
    id_type::{
        CustomerId, InvoiceId, MerchantConnectorAccountId, MerchantId, PaymentId, ProfileId,
        SubscriptionId,
    },
    types::{MinorUnit, Url},
};
use masking::Secret;
use utoipa::ToSchema;

use crate::{
    enums::{
        AuthenticationType, CaptureMethod, Currency, FutureUsage, IntentStatus, PaymentExperience,
        PaymentMethod, PaymentMethodType, PaymentType,
    },
    mandates::RecurringDetails,
    payments::{Address, NextActionData, PaymentMethodDataRequest},
};

/// Request payload for creating a subscription.
///
/// This struct captures details required to create a subscription,
/// including plan, profile, merchant connector, and optional customer info.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateSubscriptionRequest {
    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,

    /// Identifier for the associated item_price_id for the subscription.
    pub item_price_id: String,

    /// Identifier for the subscription plan.
    pub plan_id: Option<String>,

    /// Optional coupon code applied to the subscription.
    pub coupon_code: Option<String>,

    /// customer ID associated with this subscription.
    pub customer_id: CustomerId,

    /// payment details for the subscription.
    pub payment_details: CreateSubscriptionPaymentDetails,

    /// billing address for the subscription.
    pub billing: Option<Address>,

    /// shipping address for the subscription.
    pub shipping: Option<Address>,
}

/// Response payload returned after successfully creating a subscription.
///
/// Includes details such as subscription ID, status, plan, merchant, and customer info.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct SubscriptionResponse {
    /// Unique identifier for the subscription.
    pub id: SubscriptionId,

    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,

    /// Current status of the subscription.
    pub status: SubscriptionStatus,

    /// Identifier for the associated subscription plan.
    pub plan_id: Option<String>,

    /// Identifier for the associated item_price_id for the subscription.
    pub item_price_id: Option<String>,

    /// Associated profile ID.
    pub profile_id: ProfileId,

    #[schema(value_type = Option<String>)]
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    pub client_secret: Option<Secret<String>>,

    /// Merchant identifier owning this subscription.
    pub merchant_id: MerchantId,

    /// Optional coupon code applied to this subscription.
    pub coupon_code: Option<String>,

    /// Optional customer ID associated with this subscription.
    pub customer_id: CustomerId,

    /// Payment details for the invoice.
    pub payment: Option<PaymentResponseData>,

    /// Invoice Details for the subscription.
    pub invoice: Option<Invoice>,
}

/// Possible states of a subscription lifecycle.
///
/// - `Created`: Subscription was created but not yet activated.
/// - `Active`: Subscription is currently active.
/// - `InActive`: Subscription is inactive.
/// - `Pending`: Subscription is pending activation.
/// - `Trial`: Subscription is in a trial period.
/// - `Paused`: Subscription is paused.
/// - `Unpaid`: Subscription is unpaid.
/// - `Onetime`: Subscription is a one-time payment.
/// - `Cancelled`: Subscription has been cancelled.
/// - `Failed`: Subscription has failed.
#[derive(Debug, Clone, serde::Serialize, strum::EnumString, strum::Display, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// Subscription is active.
    Active,
    /// Subscription is created but not yet active.
    Created,
    /// Subscription is inactive.
    InActive,
    /// Subscription is in pending state.
    Pending,
    /// Subscription is in trial state.
    Trial,
    /// Subscription is paused.
    Paused,
    /// Subscription is unpaid.
    Unpaid,
    /// Subscription is a one-time payment.
    Onetime,
    /// Subscription is cancelled.
    Cancelled,
    /// Subscription has failed.
    Failed,
}

impl SubscriptionResponse {
    /// Creates a new [`CreateSubscriptionResponse`] with the given identifiers.
    ///
    /// By default, `client_secret`, `coupon_code`, and `customer` fields are `None`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: SubscriptionId,
        merchant_reference_id: Option<String>,
        status: SubscriptionStatus,
        plan_id: Option<String>,
        item_price_id: Option<String>,
        profile_id: ProfileId,
        merchant_id: MerchantId,
        client_secret: Option<Secret<String>>,
        customer_id: CustomerId,
        payment: Option<PaymentResponseData>,
        invoice: Option<Invoice>,
    ) -> Self {
        Self {
            id,
            merchant_reference_id,
            status,
            plan_id,
            item_price_id,
            profile_id,
            client_secret,
            merchant_id,
            coupon_code: None,
            customer_id,
            payment,
            invoice,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct GetPlansResponse {
    pub plan_id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_id: Vec<SubscriptionPlanPrices>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct SubscriptionPlanPrices {
    pub price_id: String,
    pub plan_id: Option<String>,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub interval: PeriodUnit,
    pub interval_count: i64,
    pub trial_period: Option<i64>,
    pub trial_period_unit: Option<PeriodUnit>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub enum PeriodUnit {
    Day,
    Week,
    Month,
    Year,
}

/// For Client based calls, SDK will use the client_secret\nin order to call /payment_methods\nClient secret will be generated whenever a new\npayment method is created
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ClientSecret(String);

impl ClientSecret {
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_string(&self) -> &String {
        &self.0
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, ToSchema)]
pub struct GetPlansQuery {
    #[schema(value_type = Option<String>)]
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    pub client_secret: Option<ClientSecret>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl ApiEventMetric for SubscriptionResponse {}
impl ApiEventMetric for CreateSubscriptionRequest {}
impl ApiEventMetric for GetPlansQuery {}
impl ApiEventMetric for GetPlansResponse {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ConfirmSubscriptionPaymentDetails {
    pub shipping: Option<Address>,
    pub billing: Option<Address>,
    pub payment_method: PaymentMethod,
    pub payment_method_type: Option<PaymentMethodType>,
    pub payment_method_data: PaymentMethodDataRequest,
    pub customer_acceptance: Option<CustomerAcceptance>,
    pub payment_type: Option<PaymentType>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateSubscriptionPaymentDetails {
    /// The url to which user must be redirected to after completion of the purchase
    #[schema(value_type = String)]
    pub return_url: Url,
    pub setup_future_usage: Option<FutureUsage>,
    pub capture_method: Option<CaptureMethod>,
    pub authentication_type: Option<AuthenticationType>,
    pub payment_type: Option<PaymentType>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentDetails {
    pub payment_method: Option<PaymentMethod>,
    pub payment_method_type: Option<PaymentMethodType>,
    pub payment_method_data: Option<PaymentMethodDataRequest>,
    pub setup_future_usage: Option<FutureUsage>,
    pub customer_acceptance: Option<CustomerAcceptance>,
    /// The url to which user must be redirected to after completion of the purchase
    #[schema(value_type = Option<String>)]
    pub return_url: Option<Url>,
    pub capture_method: Option<CaptureMethod>,
    pub authentication_type: Option<AuthenticationType>,
    pub payment_type: Option<PaymentType>,
}

// Creating new type for PaymentRequest API call as usage of api_models::PaymentsRequest will result in invalid payment request during serialization
// Eg: Amount will be serialized as { amount: {Value: 100 }}
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CreatePaymentsRequestData {
    pub amount: MinorUnit,
    pub currency: Currency,
    pub customer_id: Option<CustomerId>,
    pub billing: Option<Address>,
    pub shipping: Option<Address>,
    pub profile_id: Option<ProfileId>,
    pub setup_future_usage: Option<FutureUsage>,
    /// The url to which user must be redirected to after completion of the purchase
    #[schema(value_type = Option<String>)]
    pub return_url: Option<Url>,
    pub capture_method: Option<CaptureMethod>,
    pub authentication_type: Option<AuthenticationType>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct ConfirmPaymentsRequestData {
    pub billing: Option<Address>,
    pub shipping: Option<Address>,
    pub profile_id: Option<ProfileId>,
    pub payment_method: PaymentMethod,
    pub payment_method_type: Option<PaymentMethodType>,
    pub payment_method_data: PaymentMethodDataRequest,
    pub customer_acceptance: Option<CustomerAcceptance>,
    pub payment_type: Option<PaymentType>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CreateAndConfirmPaymentsRequestData {
    pub amount: MinorUnit,
    pub currency: Currency,
    pub customer_id: Option<CustomerId>,
    pub confirm: bool,
    pub billing: Option<Address>,
    pub shipping: Option<Address>,
    pub profile_id: Option<ProfileId>,
    pub setup_future_usage: Option<FutureUsage>,
    /// The url to which user must be redirected to after completion of the purchase
    #[schema(value_type = Option<String>)]
    pub return_url: Option<Url>,
    pub capture_method: Option<CaptureMethod>,
    pub authentication_type: Option<AuthenticationType>,
    pub payment_method: Option<PaymentMethod>,
    pub payment_method_type: Option<PaymentMethodType>,
    pub payment_method_data: Option<PaymentMethodDataRequest>,
    pub customer_acceptance: Option<CustomerAcceptance>,
    pub payment_type: Option<PaymentType>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentResponseData {
    pub payment_id: PaymentId,
    pub status: IntentStatus,
    pub amount: MinorUnit,
    pub currency: Currency,
    pub profile_id: Option<ProfileId>,
    pub connector: Option<String>,
    /// Identifier for Payment Method
    #[schema(value_type = Option<String>, example = "pm_01926c58bc6e77c09e809964e72af8c8")]
    pub payment_method_id: Option<Secret<String>>,
    /// The url to which user must be redirected to after completion of the purchase
    #[schema(value_type = Option<String>)]
    pub return_url: Option<Url>,
    pub next_action: Option<NextActionData>,
    pub payment_experience: Option<PaymentExperience>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub payment_method_type: Option<PaymentMethodType>,
    #[schema(value_type = Option<String>)]
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    pub client_secret: Option<Secret<String>>,
    pub billing: Option<Address>,
    pub shipping: Option<Address>,
    pub payment_type: Option<PaymentType>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateMitPaymentRequestData {
    pub amount: MinorUnit,
    pub currency: Currency,
    pub confirm: bool,
    pub customer_id: Option<CustomerId>,
    pub recurring_details: Option<RecurringDetails>,
    pub off_session: Option<bool>,
    pub profile_id: Option<ProfileId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ConfirmSubscriptionRequest {
    #[schema(value_type = Option<String>)]
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    pub client_secret: Option<ClientSecret>,

    /// Payment details for the invoice.
    pub payment_details: ConfirmSubscriptionPaymentDetails,
}

impl ConfirmSubscriptionRequest {
    pub fn get_billing_address(&self) -> Option<Address> {
        self.payment_details
            .payment_method_data
            .billing
            .as_ref()
            .or(self.payment_details.billing.as_ref())
            .cloned()
    }
}

impl ApiEventMetric for ConfirmSubscriptionRequest {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateAndConfirmSubscriptionRequest {
    /// Identifier for the associated plan_id.
    pub plan_id: Option<String>,

    /// Identifier for the associated item_price_id for the subscription.
    pub item_price_id: String,

    /// Identifier for the coupon code for the subscription.
    pub coupon_code: Option<String>,

    /// Identifier for customer.
    pub customer_id: CustomerId,

    /// Billing address for the subscription.
    pub billing: Option<Address>,

    /// Shipping address for the subscription.
    pub shipping: Option<Address>,

    /// Payment details for the invoice.
    pub payment_details: PaymentDetails,

    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,
}

impl CreateAndConfirmSubscriptionRequest {
    pub fn get_billing_address(&self) -> Option<Address> {
        self.payment_details
            .payment_method_data
            .as_ref()
            .and_then(|data| data.billing.clone())
            .or(self.billing.clone())
    }
}

impl ApiEventMetric for CreateAndConfirmSubscriptionRequest {}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct ConfirmSubscriptionResponse {
    /// Unique identifier for the subscription.
    pub id: SubscriptionId,

    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,

    /// Current status of the subscription.
    pub status: SubscriptionStatus,

    /// Identifier for the associated subscription plan.
    pub plan_id: Option<String>,

    /// Identifier for the associated item_price_id for the subscription.
    pub item_price_id: Option<String>,

    /// Optional coupon code applied to this subscription.
    pub coupon: Option<String>,

    /// Associated profile ID.
    pub profile_id: ProfileId,

    /// Payment details for the invoice.
    pub payment: Option<PaymentResponseData>,

    /// Customer ID associated with this subscription.
    pub customer_id: Option<CustomerId>,

    /// Invoice Details for the subscription.
    pub invoice: Option<Invoice>,

    /// Billing Processor subscription ID.
    pub billing_processor_subscription_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct Invoice {
    /// Unique identifier for the invoice.
    pub id: InvoiceId,

    /// Unique identifier for the subscription.
    pub subscription_id: SubscriptionId,

    /// Identifier for the merchant.
    pub merchant_id: MerchantId,

    /// Identifier for the profile.
    pub profile_id: ProfileId,

    /// Identifier for the merchant connector account.
    pub merchant_connector_id: MerchantConnectorAccountId,

    /// Identifier for the Payment.
    pub payment_intent_id: Option<PaymentId>,

    /// Identifier for Payment Method
    #[schema(value_type = Option<String>, example = "pm_01926c58bc6e77c09e809964e72af8c8")]
    pub payment_method_id: Option<String>,

    /// Identifier for the Customer.
    pub customer_id: CustomerId,

    /// Invoice amount.
    pub amount: MinorUnit,

    /// Currency for the invoice payment.
    pub currency: Currency,

    /// Status of the invoice.
    pub status: InvoiceStatus,
}

impl ApiEventMetric for ConfirmSubscriptionResponse {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UpdateSubscriptionRequest {
    /// Identifier for the associated plan_id.
    pub plan_id: String,
    /// Identifier for the associated item_price_id for the subscription.
    pub item_price_id: String,
}

impl ApiEventMetric for UpdateSubscriptionRequest {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct EstimateSubscriptionQuery {
    /// Identifier for the associated subscription plan.
    pub plan_id: Option<String>,

    /// Identifier for the associated item_price_id for the subscription.
    pub item_price_id: String,

    /// Identifier for the coupon code for the subscription.
    pub coupon_code: Option<String>,
}

impl ApiEventMetric for EstimateSubscriptionQuery {}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct EstimateSubscriptionResponse {
    /// Estimated amount to be charged for the invoice.
    pub amount: MinorUnit,
    /// Currency for the amount.
    pub currency: Currency,
    /// Identifier for the associated plan_id.
    pub plan_id: Option<String>,
    /// Identifier for the associated item_price_id for the subscription.
    pub item_price_id: Option<String>,
    /// Identifier for the coupon code for the subscription.
    pub coupon_code: Option<String>,
    /// Identifier for customer.
    pub customer_id: Option<CustomerId>,
    pub line_items: Vec<SubscriptionLineItem>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct SubscriptionLineItem {
    /// Unique identifier for the line item.
    pub item_id: String,
    /// Type of the line item.
    pub item_type: String,
    /// Description of the line item.
    pub description: String,
    /// Amount for the line item.
    pub amount: MinorUnit,
    /// Currency for the line item
    pub currency: Currency,
    /// Quantity of the line item.
    pub quantity: i64,
}

impl ApiEventMetric for EstimateSubscriptionResponse {}
