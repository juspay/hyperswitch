use api_models::analytics::{
    payments::{PaymentDimensions, PaymentMetrics},
    refunds::{RefundDimensions, RefundMetrics},
    NameDescription,
};
use strum::IntoEnumIterator;

pub fn get_payment_dimensions() -> Vec<NameDescription> {
    PaymentDimensions::iter().map(Into::into).collect()
}

pub fn get_refund_dimensions() -> Vec<NameDescription> {
    RefundDimensions::iter().map(Into::into).collect()
}

pub fn get_payment_metrics_info() -> Vec<NameDescription> {
    PaymentMetrics::iter().map(Into::into).collect()
}

pub fn get_refund_metrics_info() -> Vec<NameDescription> {
    RefundMetrics::iter().map(Into::into).collect()
}
