// Re-export types that have been moved to common_types
pub use common_types::{
    payment_intent_types::{DefaultTax, OrderDetailsWithAmount, PaymentMethodTypeTax, TaxDetails},
    storage_types::{
        ApplePayRecurringDetails, ApplePayRegularBillingDetails, BoletoAdditionalDetails,
        FeatureMetadata, ImmediateExpirationTime, PixAdditionalDetails,
        RecurringPaymentIntervalUnit, RedirectResponse, ScheduledExpirationTime,
    },
};

#[cfg(feature = "v2")]
pub use common_types::storage_types::{
    BillingConnectorAdditionalCardInfo, BillingConnectorPaymentDetails,
    BillingConnectorPaymentMethodDetails, PaymentRevenueRecoveryMetadata,
};
