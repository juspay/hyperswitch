use api_models::analytics::{
    api_event::{ApiEventDimensions, ApiEventMetrics},
    payments::{PaymentDimensions, PaymentMetrics},
    refunds::{RefundDimensions, RefundMetrics},
    sdk_events::{SdkEventDimensions, SdkEventMetrics},
    NameDescription,
};
use strum::IntoEnumIterator;

/// Returns a vector of NameDescription items representing the payment dimensions available.
/// This method iterates through the PaymentDimensions enum and maps each item into a NameDescription struct,
/// then collects all the mapped items into a vector and returns it.
pub fn get_payment_dimensions() -> Vec<NameDescription> {
    PaymentDimensions::iter().map(Into::into).collect()
}

/// Returns a vector of NameDescription elements containing the refund dimensions by iterating through the RefundDimensions enum and converting each element into a NameDescription.
pub fn get_refund_dimensions() -> Vec<NameDescription> {
    RefundDimensions::iter().map(Into::into).collect()
}

/// Returns a vector of NameDescription structs representing the dimensions of an SDK event. 
pub fn get_sdk_event_dimensions() -> Vec<NameDescription> {
    SdkEventDimensions::iter().map(Into::into).collect()
}

/// Retrieves the dimensions of API events.
pub fn get_api_event_dimensions() -> Vec<NameDescription> {
    ApiEventDimensions::iter().map(Into::into).collect()
}

/// Retrieves payment metrics information by iterating through all the available payment metrics, converting them into NameDescription format, and collecting them into a vector.
pub fn get_payment_metrics_info() -> Vec<NameDescription> {
    PaymentMetrics::iter().map(Into::into).collect()
}

/// Returns a vector of NameDescription structs containing information about refund metrics.
pub fn get_refund_metrics_info() -> Vec<NameDescription> {
    RefundMetrics::iter().map(Into::into).collect()
}

/// Retrieves the metrics information for SDK events.
/// 
/// This method iterates through all the SDK event metrics and converts them into a vector of NameDescription structs.
/// 
/// # Returns
/// 
/// A vector of NameDescription structs containing the metrics information for SDK events.
pub fn get_sdk_event_metrics_info() -> Vec<NameDescription> {
    SdkEventMetrics::iter().map(Into::into).collect()
}

/// Retrieves the information about API event metrics by iterating through all available metrics and converting them into a vector of NameDescription structs.
pub fn get_api_event_metrics_info() -> Vec<NameDescription> {
    ApiEventMetrics::iter().map(Into::into).collect()
}
