use common_utils::{errors as common_utils_errors, request};
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data;
use masking;
use router_env::{logger, tracing};

use crate::{api_client, consts, errors, types};
/// Trait for converting from one foreign type to another
pub trait ForeignTryFrom<F>: Sized {
    /// Custom error for conversion failure
    type Error;
    /// Convert from a foreign type to the current type and return an error if the conversion fails
    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

/// Data structure to hold comparison data between Hyperswitch and UCS
#[derive(serde::Serialize, Debug)]
pub struct ComparisonData {
    /// Hyperswitch router data
    pub hyperswitch_data: masking::Secret<serde_json::Value>,
    /// Unified Connector Service router data
    pub unified_connector_service_data: masking::Secret<serde_json::Value>,
}

/// Trait to get comparison service configuration
pub trait GetComparisonServiceConfig {
    /// Retrieve the comparison service configuration if available
    fn get_comparison_service_config(&self) -> Option<types::ComparisonServiceConfig>;
}

/// Generic function to serialize router data and send comparison to external service
/// Works for both payments and refunds
pub async fn serialize_router_data_and_send_to_comparison_service<F, RouterDReq, RouterDResp>(
    state: &dyn api_client::ApiClientWrapper,
    hyperswitch_router_data: router_data::RouterData<F, RouterDReq, RouterDResp>,
    unified_connector_service_router_data: router_data::RouterData<F, RouterDReq, RouterDResp>,
    comparison_service_config: types::ComparisonServiceConfig,
    request_id: Option<String>,
) -> common_utils_errors::CustomResult<(), errors::HttpClientError>
where
    F: Send + Clone + Sync + 'static,
    RouterDReq: Send + Sync + Clone + 'static + serde::Serialize,
    RouterDResp: Send + Sync + Clone + 'static + serde::Serialize,
{
    logger::info!("Simulating UCS call for shadow mode comparison");

    let [hyperswitch_data, unified_connector_service_data] = [
        (hyperswitch_router_data, "hyperswitch"),
        (unified_connector_service_router_data, "ucs"),
    ]
    .map(|(data, source)| {
        serde_json::to_value(data)
            .map(masking::Secret::new)
            .unwrap_or_else(|e| {
                masking::Secret::new(serde_json::json!({
                    "error": e.to_string(),
                    "source": source
                }))
            })
    });

    let comparison_data = ComparisonData {
        hyperswitch_data,
        unified_connector_service_data,
    };
    let _ = send_comparison_data(
        state,
        comparison_data,
        comparison_service_config,
        request_id,
    )
    .await
    .map_err(|e| {
        logger::debug!("Failed to send comparison data: {:?}", e);
    });
    Ok(())
}

/// Sends router data comparison to external service
pub async fn send_comparison_data(
    state: &dyn api_client::ApiClientWrapper,
    comparison_data: ComparisonData,
    comparison_service_config: types::ComparisonServiceConfig,
    request_id: Option<String>,
) -> common_utils_errors::CustomResult<(), errors::HttpClientError> {
    let mut request = request::RequestBuilder::new()
        .method(request::Method::Post)
        .url(comparison_service_config.url.get_string_repr())
        .header(consts::CONTENT_TYPE, "application/json")
        .header(consts::X_FLOW_NAME, "router-data")
        .set_body(request::RequestContent::Json(Box::new(comparison_data)))
        .build();

    if let Some(req_id) = request_id {
        request.add_header(consts::X_REQUEST_ID, masking::Maskable::Normal(req_id));
    }

    let _ = state
        .get_api_client()
        .send_request(
            state,
            request,
            comparison_service_config.timeout_secs,
            false,
        )
        .await
        .inspect_err(|e| {
            tracing::debug!("Error sending comparison data: {:?}", e);
        })
        .change_context(errors::HttpClientError::RequestNotSent(
            "Failed to send comparison data to comparison service".to_string(),
        ))?;

    Ok(())
}
