pub mod payment_connector_required_fields;
pub mod settings;

use serde::Deserialize;
use url::Url;

/// Microservice configuration for payment method flows.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct MicroServicesConfig {
    /// Base URL for the modular payment methods service.
    pub payment_methods_base_url: ModularPaymentMethodServiceUrl,
}

/// Base URL wrapper for the modular payment methods service.
#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
pub struct ModularPaymentMethodServiceUrl(pub Url);

impl Default for ModularPaymentMethodServiceUrl {
    fn default() -> Self {
        Self(
            #[allow(clippy::expect_used)]
            Url::parse("http://localhost:8080")
                .expect("Failed to parse default payment_methods_base_url"),
        )
    }
}

impl std::ops::Deref for ModularPaymentMethodServiceUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Url> for ModularPaymentMethodServiceUrl {
    fn as_ref(&self) -> &Url {
        &self.0
    }
}
