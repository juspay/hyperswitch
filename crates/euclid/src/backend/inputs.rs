use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::enums;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MandateData {
    pub mandate_acceptance_type: Option<enums::MandateAcceptanceType>,
    pub mandate_type: Option<enums::MandateType>,
    pub payment_type: Option<enums::PaymentType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodInput {
    pub payment_method: Option<enums::PaymentMethod>,
    pub payment_method_type: Option<enums::PaymentMethodType>,
    pub card_network: Option<enums::CardNetwork>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInput {
    pub amount: i64,
    pub currency: enums::Currency,
    pub authentication_type: Option<enums::AuthenticationType>,
    pub card_bin: Option<String>,
    pub capture_method: Option<enums::CaptureMethod>,
    pub business_country: Option<enums::Country>,
    pub billing_country: Option<enums::Country>,
    pub business_label: Option<String>,
    pub setup_future_usage: Option<enums::SetupFutureUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInput {
    pub metadata: Option<FxHashMap<String, String>>,
    pub payment: PaymentInput,
    pub payment_method: PaymentMethodInput,
    pub mandate: MandateData,
}
