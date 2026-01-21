pub mod payment_connector_required_fields;
pub mod settings;

use hyperswitch_interfaces::configs::ModularPaymentMethodServiceUrl;
use serde::Deserialize;

/// Microservice configuration for payment method flows.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct MicroServicesConfig {
    /// Base URL for the modular payment methods service.
    pub payment_methods_base_url: ModularPaymentMethodServiceUrl,
}
