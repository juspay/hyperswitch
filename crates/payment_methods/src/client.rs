//! Payment method microservice flows and lightweight client context.

/// Create payment method flow.
pub mod create;
/// Delete payment method flow.
pub mod delete;
/// List customer payment methods flow.
#[cfg(feature = "v1")]
pub mod list;
#[cfg(feature = "v1")]
/// Retrieve payment method flow.
pub mod retrieve;
/// Update payment method flow.
pub mod update;

use common_utils::request::Headers;
pub use create::{CreatePaymentMethod, CreatePaymentMethodV1Request};
pub use delete::{DeletePaymentMethod, DeletePaymentMethodV1Request};
use hyperswitch_interfaces::micro_service::MicroserviceClient;
#[cfg(feature = "v1")]
pub use retrieve::{RetrievePaymentMethod, RetrievePaymentMethodV1Request};
use router_env::RequestIdentifier;
pub use update::{UpdatePaymentMethod, UpdatePaymentMethodV1Request};

use crate::configs::ModularPaymentMethodServiceUrl;

#[derive(Debug)]
/// Lightweight client context for payment method microservice calls.
pub struct PaymentMethodClient<'a> {
    /// Base URL for the payment method service.
    pub base_url: &'a ModularPaymentMethodServiceUrl,
    /// Parent headers to propagate to the microservice.
    pub parent_headers: &'a Headers,
    /// Trace configuration for request correlation.
    pub trace: &'a RequestIdentifier,
}

impl<'a> PaymentMethodClient<'a> {
    /// Create a new client with base URL, parent headers, and trace configuration.
    pub fn new(
        base_url: &'a ModularPaymentMethodServiceUrl,
        parent_headers: &'a Headers,
        trace: &'a RequestIdentifier,
    ) -> Self {
        Self {
            base_url,
            parent_headers,
            trace,
        }
    }
}

impl<'a> MicroserviceClient for PaymentMethodClient<'a> {
    fn base_url(&self) -> &url::Url {
        self.base_url.as_ref()
    }

    fn parent_headers(&self) -> &Headers {
        self.parent_headers
    }

    fn trace(&self) -> &RequestIdentifier {
        self.trace
    }
}
