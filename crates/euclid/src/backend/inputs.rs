use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    enums,
    frontend::dir::enums::{CustomerDeviceDisplaySize, CustomerDevicePlatform, CustomerDeviceType},
};

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
    pub amount: common_utils::types::MinorUnit,
    pub currency: enums::Currency,
    pub authentication_type: Option<enums::AuthenticationType>,
    pub card_bin: Option<String>,
    pub extended_card_bin: Option<String>,
    pub capture_method: Option<enums::CaptureMethod>,
    pub business_country: Option<enums::Country>,
    pub billing_country: Option<enums::Country>,
    pub business_label: Option<String>,
    pub setup_future_usage: Option<enums::SetupFutureUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquirerDataInput {
    pub country: Option<enums::Country>,
    pub fraud_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerDeviceDataInput {
    pub platform: Option<CustomerDevicePlatform>,
    pub device_type: Option<CustomerDeviceType>,
    pub display_size: Option<CustomerDeviceDisplaySize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuerDataInput {
    pub name: Option<String>,
    pub country: Option<enums::Country>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInput {
    pub metadata: Option<FxHashMap<String, String>>,
    pub payment: PaymentInput,
    pub payment_method: PaymentMethodInput,
    pub acquirer_data: Option<AcquirerDataInput>,
    pub customer_device_data: Option<CustomerDeviceDataInput>,
    pub issuer_data: Option<IssuerDataInput>,
    pub mandate: MandateData,
}
