use common_utils::{
    consts::{X_CONNECTOR_NAME, X_SUB_FLOW_NAME},
    errors as common_utils_errors, request,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::router_data;
use hyperswitch_masking;
use router_env::{logger, tracing};

use crate::{api_client, consts, errors, types};
/// Trait for converting from one foreign type to another
pub trait ForeignTryFrom<F>: Sized {
    /// Custom error for conversion failure
    type Error;
    /// Convert from a foreign type to the current type and return an error if the conversion fails
    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

/// Trait for infallibly converting from one foreign type to another
pub trait ForeignFrom<F>: Sized {
    /// Convert from a foreign type to the current type
    fn foreign_from(from: F) -> Self;
}

/// Data structure to hold comparison data between Hyperswitch and UCS
#[derive(serde::Serialize, Debug)]
pub struct ComparisonData {
    /// Hyperswitch router data
    pub hyperswitch_data: hyperswitch_masking::Secret<serde_json::Value>,
    /// Unified Connector Service router data
    pub unified_connector_service_data: hyperswitch_masking::Secret<serde_json::Value>,
}

/// Trait to get comparison service configuration
pub trait GetComparisonServiceConfig {
    /// Retrieve the comparison service configuration if available
    fn get_comparison_service_config(&self) -> Option<types::ComparisonServiceConfig>;
}

/// Serialize `Result`-shaped router data for both sides and forward to the validation
/// (comparison) service. Covers all four quadrants — (Ok,Ok), (Ok,Err), (Err,Ok), (Err,Err) —
/// so error cases are still visible as a diff. No-op when the comparison service is not
/// configured. Errors from the comparison service itself are logged and swallowed so they
/// cannot affect the caller.
pub async fn serialize_comparison_results_and_send<S, F, RouterDReq, RouterDResp>(
    state: &S,
    connector_name: String,
    hyperswitch_result: Result<router_data::RouterData<F, RouterDReq, RouterDResp>, String>,
    unified_connector_service_result: Result<
        router_data::RouterData<F, RouterDReq, RouterDResp>,
        String,
    >,
) where
    S: api_client::ApiClientWrapper + GetComparisonServiceConfig,
    F: Send + Clone + Sync + 'static,
    RouterDReq: Send + Sync + Clone + 'static + serde::Serialize,
    RouterDResp: Send + Sync + Clone + 'static + serde::Serialize,
{
    let Some(comparison_service_config) = state.get_comparison_service_config() else {
        return;
    };

    let to_value = |res: Result<router_data::RouterData<F, RouterDReq, RouterDResp>, String>,
                    side: &str| {
        match res {
            Ok(rd) => serde_json::to_value(rd).unwrap_or_else(|e| {
                serde_json::json!({
                    "error": format!("serialize {side} router_data failed: {e}")
                })
            }),
            Err(e) => serde_json::json!({ "error": e }),
        }
    };

    let comparison_data = ComparisonData {
        hyperswitch_data: hyperswitch_masking::Secret::new(to_value(
            hyperswitch_result,
            "hyperswitch",
        )),
        unified_connector_service_data: hyperswitch_masking::Secret::new(to_value(
            unified_connector_service_result,
            "ucs",
        )),
    };

    let sub_flow_name = api_client::get_flow_name::<F>().ok();
    let request_id = state.get_request_id_str();
    let _ = send_comparison_data(
        state,
        comparison_data,
        comparison_service_config,
        connector_name,
        sub_flow_name,
        request_id,
    )
    .await
    .inspect_err(|e| logger::debug!("Failed to send comparison data: {:?}", e));
}

/// Sends router data comparison to external service
pub async fn send_comparison_data(
    state: &dyn api_client::ApiClientWrapper,
    comparison_data: ComparisonData,
    comparison_service_config: types::ComparisonServiceConfig,
    connector_name: String,
    sub_flow_name: Option<String>,
    request_id: Option<String>,
) -> common_utils_errors::CustomResult<(), errors::HttpClientError> {
    let mut request = request::RequestBuilder::new()
        .method(request::Method::Post)
        .url(comparison_service_config.url.get_string_repr())
        .header(consts::CONTENT_TYPE, "application/json")
        .header(consts::X_FLOW_NAME, "router-data")
        .set_body(request::RequestContent::Json(Box::new(comparison_data)))
        .build();

    request.add_header(
        X_CONNECTOR_NAME,
        hyperswitch_masking::Maskable::Normal(connector_name),
    );

    if let Some(sub_flow_name) = sub_flow_name.filter(|name| !name.is_empty()) {
        request.add_header(
            X_SUB_FLOW_NAME,
            hyperswitch_masking::Maskable::Normal(sub_flow_name),
        );
    }

    if let Some(req_id) = request_id {
        request.add_header(
            consts::X_REQUEST_ID,
            hyperswitch_masking::Maskable::Normal(req_id),
        );
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
