use api_models::analytics::{
    api_event::{ApiEventDimensions, ApiEventMetrics},
    auth_events::AuthEventMetrics,
    disputes::{DisputeDimensions, DisputeMetrics},
    frm::{FrmDimensions, FrmMetrics},
    payment_intents::{PaymentIntentDimensions, PaymentIntentMetrics},
    payments::{PaymentDimensions, PaymentMetrics},
    refunds::{RefundDimensions, RefundMetrics},
    sdk_events::{SdkEventDimensions, SdkEventMetrics},
    NameDescription,
};
use strum::IntoEnumIterator;

pub fn get_payment_dimensions() -> Vec<NameDescription> {
    PaymentDimensions::iter().map(Into::into).collect()
}

pub fn get_payment_intent_dimensions() -> Vec<NameDescription> {
    PaymentIntentDimensions::iter().map(Into::into).collect()
}

pub fn get_refund_dimensions() -> Vec<NameDescription> {
    RefundDimensions::iter().map(Into::into).collect()
}

pub fn get_frm_dimensions() -> Vec<NameDescription> {
    FrmDimensions::iter().map(Into::into).collect()
}

pub fn get_sdk_event_dimensions() -> Vec<NameDescription> {
    SdkEventDimensions::iter().map(Into::into).collect()
}

pub fn get_api_event_dimensions() -> Vec<NameDescription> {
    ApiEventDimensions::iter().map(Into::into).collect()
}

pub fn get_payment_metrics_info() -> Vec<NameDescription> {
    PaymentMetrics::iter().map(Into::into).collect()
}

pub fn get_payment_intent_metrics_info() -> Vec<NameDescription> {
    PaymentIntentMetrics::iter().map(Into::into).collect()
}

pub fn get_refund_metrics_info() -> Vec<NameDescription> {
    RefundMetrics::iter().map(Into::into).collect()
}

pub fn get_frm_metrics_info() -> Vec<NameDescription> {
    FrmMetrics::iter().map(Into::into).collect()
}

pub fn get_sdk_event_metrics_info() -> Vec<NameDescription> {
    SdkEventMetrics::iter().map(Into::into).collect()
}

pub fn get_auth_event_metrics_info() -> Vec<NameDescription> {
    AuthEventMetrics::iter().map(Into::into).collect()
}

pub fn get_api_event_metrics_info() -> Vec<NameDescription> {
    ApiEventMetrics::iter().map(Into::into).collect()
}

pub fn get_dispute_metrics_info() -> Vec<NameDescription> {
    DisputeMetrics::iter().map(Into::into).collect()
}

pub fn get_dispute_dimensions() -> Vec<NameDescription> {
    DisputeDimensions::iter().map(Into::into).collect()
}
