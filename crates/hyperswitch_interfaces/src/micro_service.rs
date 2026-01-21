//! Generic microservice client framework (typestate pipeline + macro helpers).
//!
//! This module is microservice-agnostic. Domain-specific flows and lightweight clients should
//! live in their own crates and implement [`MicroserviceClientContext`].
//!
//! # Examples
//!
//! ```rust,ignore
//! use common_utils::request::Headers;
//! use hyperswitch_interfaces::{
//!     api_client::ApiClientWrapper,
//!     micro_service::{MicroserviceClientContext, MicroserviceClientError},
//! };
//! use router_env::RequestIdentifier;
//! use url::Url;
//!
//! struct ExampleClient<'a> {
//!     base_url: &'a Url,
//!     parent_headers: &'a Headers,
//!     trace: &'a RequestIdentifier,
//! }
//!
//! impl<'a> MicroserviceClientContext for ExampleClient<'a> {
//!     fn base_url(&self) -> &Url { self.base_url }
//!     fn parent_headers(&self) -> &Headers { self.parent_headers }
//!     fn trace(&self) -> &RequestIdentifier { self.trace }
//! }
//!
//! async fn call_flow(
//!     state: &dyn ApiClientWrapper,
//!     client: &ExampleClient<'_>,
//! ) -> Result<(), MicroserviceClientError> {
//!     // ExampleFlow::call(state, client, flow).await?;
//!     Ok(())
//! }
//! ```

mod error;
mod executor;
mod macros;
mod state;

use common_utils::request::Headers;
pub use error::{MicroserviceClientError, MicroserviceClientErrorKind};
pub use executor::execute_microservice_operation;
use router_env::RequestIdentifier;
pub use state::{ClientOperation, Executed, TransformedRequest, TransformedResponse, Validated};
use url::Url;

/// Minimal context required to execute a microservice flow.
///
/// Implement this for lightweight client wrappers that carry base URL, headers, and trace config.
pub trait MicroserviceClientContext {
    /// Base URL for the microservice.
    fn base_url(&self) -> &Url;
    /// Parent headers to forward to the microservice.
    fn parent_headers(&self) -> &Headers;
    /// Trace identifier configuration.
    fn trace(&self) -> &RequestIdentifier;
}
