/// Payment method microservice flows and client types.
///
/// # Examples
///
/// ```rust
/// use common_utils::request::Headers;
/// use hyperswitch_interfaces::{
///     api_client::ApiClientWrapper,
///     configs::ModularPaymentMethodServiceUrl,
///     micro_service::payment_method::{CreatePaymentMethod, PaymentMethodClient},
/// };
/// use router_env::RequestIdentifier;
/// use serde_json::json;
///
/// async fn create_payment_method(
///     state: &dyn ApiClientWrapper,
///     base_url: &ModularPaymentMethodServiceUrl,
///     trace_identifier: &RequestIdentifier,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     let headers = Headers::new();
///     let client = PaymentMethodClient::new(base_url, &headers, trace_identifier);
///     let flow = CreatePaymentMethod { payload: json!({ "card": "dummy" }) };
///
///     let _response = CreatePaymentMethod::call(state, &client, flow).await?;
///     Ok(())
/// }
/// ```
pub mod payment_method;

mod error;
mod executor;
mod macros;
mod state;

pub use error::{MicroserviceClientError, MicroserviceClientErrorKind};
pub use executor::execute_microservice_operation;
pub use state::{ClientOperation, Executed, TransformedRequest, TransformedResponse, Validated};
