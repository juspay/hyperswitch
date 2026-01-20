/// Create payment method flow.
pub mod create;
/// Delete payment method flow.
pub mod delete;
/// Retrieve payment method flow.
pub mod retrieve;
/// Update payment method flow.
pub mod update;

use common_utils::request::Headers;
pub use create::CreatePaymentMethod;
pub use delete::DeletePaymentMethod;
pub use retrieve::RetrievePaymentMethod;
use router_env::RequestIdentifier;
pub use update::UpdatePaymentMethod;

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
