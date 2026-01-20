pub mod create;
pub mod delete;
pub mod retrieve;
pub mod update;

use common_utils::request::Headers;
pub use create::CreatePaymentMethod;
pub use delete::DeletePaymentMethod;
pub use retrieve::RetrievePaymentMethod;
use router_env::RequestIdentifier;
pub use update::UpdatePaymentMethod;

use crate::configs::ModularPaymentMethodServiceUrl;

pub struct PaymentMethodClient<'a> {
    pub base_url: &'a ModularPaymentMethodServiceUrl,
    pub parent_headers: &'a Headers,
    pub trace: &'a RequestIdentifier,
}

impl<'a> PaymentMethodClient<'a> {
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
