use common_types::payments::CustomerAcceptance;
use common_utils::{errors::ValidationError, events::ApiEventMetric, types::MinorUnit};
use masking::Secret;
use utoipa::ToSchema;

use crate::{
    enums as api_enums,
    payments::{Address, PaymentMethodDataRequest},
};

/// Request payload for creating a subscription.
///
/// This struct captures details required to create a subscription,
/// including plan, profile, merchant connector, and optional customer info.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateSubscriptionRequest {
    /// Amount to be charged for the invoice.
    pub amount: MinorUnit,

    /// Currency for the amount.
    pub currency: api_enums::Currency,

    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,

    /// Identifier for the subscription plan.
    pub plan_id: Option<String>,

    /// Optional coupon code applied to the subscription.
    pub coupon_code: Option<String>,

    /// customer ID associated with this subscription.
    pub customer_id: common_utils::id_type::CustomerId,

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
    pub id: common_utils::id_type::SubscriptionId,

    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,

    /// Current status of the subscription.
    pub status: SubscriptionStatus,

    /// Identifier for the associated subscription plan.
    pub plan_id: Option<String>,

    /// Associated profile ID.
    pub profile_id: common_utils::id_type::ProfileId,

    /// Optional client secret used for secure client-side interactions.
    pub client_secret: Option<Secret<String>>,

    /// Merchant identifier owning this subscription.
    pub merchant_id: common_utils::id_type::MerchantId,

    /// Optional coupon code applied to this subscription.
    pub coupon_code: Option<String>,

    /// Optional customer ID associated with this subscription.
    pub customer_id: common_utils::id_type::CustomerId,
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
        id: common_utils::id_type::SubscriptionId,
        merchant_reference_id: Option<String>,
        status: SubscriptionStatus,
        plan_id: Option<String>,
        profile_id: common_utils::id_type::ProfileId,
        merchant_id: common_utils::id_type::MerchantId,
        client_secret: Option<Secret<String>>,
        customer_id: common_utils::id_type::CustomerId,
    ) -> Self {
        Self {
            id,
            merchant_reference_id,
            status,
            plan_id,
            profile_id,
            client_secret,
            merchant_id,
            coupon_code: None,
            customer_id,
        }
    }
}

impl ApiEventMetric for SubscriptionResponse {}
impl ApiEventMetric for CreateSubscriptionRequest {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ConfirmSubscriptionPaymentDetails {
    pub payment_method: api_enums::PaymentMethod,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub payment_method_data: PaymentMethodDataRequest,
    pub customer_acceptance: Option<CustomerAcceptance>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateSubscriptionPaymentDetails {
    pub return_url: common_utils::types::Url,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub capture_method: Option<api_enums::CaptureMethod>,
    pub authentication_type: Option<api_enums::AuthenticationType>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentDetails {
    pub payment_method: Option<api_enums::PaymentMethod>,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub payment_method_data: Option<PaymentMethodDataRequest>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub customer_acceptance: Option<CustomerAcceptance>,
    pub return_url: Option<common_utils::types::Url>,
    pub capture_method: Option<api_enums::CaptureMethod>,
    pub authentication_type: Option<api_enums::AuthenticationType>,
}

// Creating new type for PaymentRequest API call as usage of api_models::PaymentsRequest will result in invalid payment request during serialization
// Eg: Amount will be serialized as { amount: {Value: 100 }}
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CreatePaymentsRequestData {
    pub amount: MinorUnit,
    pub currency: api_enums::Currency,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub billing: Option<Address>,
    pub shipping: Option<Address>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub return_url: Option<common_utils::types::Url>,
    pub capture_method: Option<api_enums::CaptureMethod>,
    pub authentication_type: Option<api_enums::AuthenticationType>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct ConfirmPaymentsRequestData {
    pub billing: Option<Address>,
    pub shipping: Option<Address>,
    pub payment_method: api_enums::PaymentMethod,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub payment_method_data: PaymentMethodDataRequest,
    pub customer_acceptance: Option<CustomerAcceptance>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CreateAndConfirmPaymentsRequestData {
    pub amount: MinorUnit,
    pub currency: api_enums::Currency,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub confirm: bool,
    pub billing: Option<Address>,
    pub shipping: Option<Address>,
    pub setup_future_usage: Option<api_enums::FutureUsage>,
    pub return_url: Option<common_utils::types::Url>,
    pub capture_method: Option<api_enums::CaptureMethod>,
    pub authentication_type: Option<api_enums::AuthenticationType>,
    pub payment_method: Option<api_enums::PaymentMethod>,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
    pub payment_method_data: Option<PaymentMethodDataRequest>,
    pub customer_acceptance: Option<CustomerAcceptance>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentResponseData {
    pub payment_id: common_utils::id_type::PaymentId,
    pub status: api_enums::IntentStatus,
    pub amount: MinorUnit,
    pub currency: api_enums::Currency,
    pub connector: Option<String>,
    pub payment_method_id: Option<Secret<String>>,
    pub payment_experience: Option<api_enums::PaymentExperience>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub payment_method_type: Option<api_enums::PaymentMethodType>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ConfirmSubscriptionRequest {
    /// Client secret for SDK based interaction.
    pub client_secret: Option<String>,

    /// Identifier for the associated plan_id.
    pub plan_id: Option<String>,

    /// Identifier for the associated item_price_id for the subscription.
    pub item_price_id: Option<String>,

    /// Idenctifier for the coupon code for the subscription.
    pub coupon_code: Option<String>,

    /// Identifier for customer.
    pub customer_id: common_utils::id_type::CustomerId,

    /// Billing address for the subscription.
    pub billing: Option<Address>,

    /// Shipping address for the subscription.
    pub shipping: Option<Address>,

    /// Payment details for the invoice.
    pub payment_details: ConfirmSubscriptionPaymentDetails,
}

impl ConfirmSubscriptionRequest {
    pub fn get_item_price_id(&self) -> Result<String, error_stack::Report<ValidationError>> {
        self.item_price_id.clone().ok_or(error_stack::report!(
            ValidationError::MissingRequiredField {
                field_name: "item_price_id".to_string()
            }
        ))
    }

    pub fn get_billing_address(&self) -> Result<Address, error_stack::Report<ValidationError>> {
        self.billing.clone().ok_or(error_stack::report!(
            ValidationError::MissingRequiredField {
                field_name: "billing".to_string()
            }
        ))
    }
}

impl ApiEventMetric for ConfirmSubscriptionRequest {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateAndConfirmSubscriptionRequest {
    /// Amount to be charged for the invoice.
    pub amount: Option<MinorUnit>,

    /// Currency for the amount.
    pub currency: Option<api_enums::Currency>,

    /// Identifier for the associated plan_id.
    pub plan_id: Option<String>,

    /// Identifier for the associated item_price_id for the subscription.
    pub item_price_id: Option<String>,

    /// Idenctifier for the coupon code for the subscription.
    pub coupon_code: Option<String>,

    /// Identifier for customer.
    pub customer_id: common_utils::id_type::CustomerId,

    /// Billing address for the subscription.
    pub billing: Option<Address>,

    /// Shipping address for the subscription.
    pub shipping: Option<Address>,

    /// Payment details for the invoice.
    pub payment_details: PaymentDetails,

    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,
}

impl ApiEventMetric for CreateAndConfirmSubscriptionRequest {}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct ConfirmSubscriptionResponse {
    /// Unique identifier for the subscription.
    pub id: common_utils::id_type::SubscriptionId,

    /// Merchant specific Unique identifier.
    pub merchant_reference_id: Option<String>,

    /// Current status of the subscription.
    pub status: SubscriptionStatus,

    /// Identifier for the associated subscription plan.
    pub plan_id: Option<String>,

    /// Identifier for the associated item_price_id for the subscription.
    pub price_id: Option<String>,

    /// Optional coupon code applied to this subscription.
    pub coupon: Option<String>,

    /// Associated profile ID.
    pub profile_id: common_utils::id_type::ProfileId,

    /// Payment details for the invoice.
    pub payment: Option<PaymentResponseData>,

    /// Customer ID associated with this subscription.
    pub customer_id: Option<common_utils::id_type::CustomerId>,

    /// Invoice Details for the subscription.
    pub invoice: Option<Invoice>,

    /// Billing Processor subscription ID.
    pub billing_processor_subscription_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct Invoice {
    /// Unique identifier for the invoice.
    pub id: common_utils::id_type::InvoiceId,

    /// Unique identifier for the subscription.
    pub subscription_id: common_utils::id_type::SubscriptionId,

    /// Identifier for the merchant.
    pub merchant_id: common_utils::id_type::MerchantId,

    /// Identifier for the profile.
    pub profile_id: common_utils::id_type::ProfileId,

    /// Identifier for the merchant connector account.
    pub merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,

    /// Identifier for the Payment.
    pub payment_intent_id: Option<common_utils::id_type::PaymentId>,

    /// Identifier for the Payment method.
    pub payment_method_id: Option<String>,

    /// Identifier for the Customer.
    pub customer_id: common_utils::id_type::CustomerId,

    /// Invoice amount.
    pub amount: MinorUnit,

    /// Currency for the invoice payment.
    pub currency: api_enums::Currency,

    /// Status of the invoice.
    pub status: String,
}

impl ApiEventMetric for ConfirmSubscriptionResponse {}
