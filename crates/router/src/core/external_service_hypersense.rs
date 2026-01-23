use api_models::external_service_hypersense as external_service_hypersense_api;
use common_utils::{
    consts,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use external_services::http_client;
use router_env::logger;

use crate::{
    core::errors::{self, RouterResponse},
    services::{
        api,
        authentication::{self},
    },
    SessionState,
};

pub async fn get_hypersense_fee_estimate(
    state: SessionState,
    api_path: String,
    query_params: &str,
    json_payload: external_service_hypersense_api::ExternalFeeEstimatePayload,
    user: authentication::UserFromToken,
) -> RouterResponse<external_service_hypersense_api::ExternalFeeEstimateResponse> {
    let base = match &state.conf.hypersense {
        Some(h) if h.enabled => &h.api_url,
        _ => {
            return Err(errors::ApiErrorResponse::InternalServerError.into());
        }
    };
    let url = format!("{}/fee-analysis/{}?{}", base, api_path, query_params);
    let merchant_id = user.merchant_id.get_string_repr().to_string();
    let profile_id = user.profile_id.get_string_repr().to_string();
    let org_id = user.org_id.get_string_repr().to_string();
    let role_id = user.role_id;
    let role_info = crate::services::authorization::roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &role_id,
        &user.org_id,
        user.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let role_scope = role_info.get_scope();
    let header_value = format!("{},{},{},{}", merchant_id, profile_id, org_id, role_scope);

    let request_builder = RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .header(consts::X_HYPERSENSE_ID, &header_value)
        .set_body(RequestContent::Json(Box::new(json_payload.payload)));

    let request = request_builder.build();

    let response = http_client::send_request(
        &state.conf.proxy,
        request,
        Some(consts::REQUEST_TIME_OUT_FOR_AI_SERVICE),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Error when sending request to Hypersense service")?;

    logger::info!(
        "Request for hypersense fee estimate service: {:?}",
        response
    );

    let status = response.status();

    // Handle 4xx responses with more specific api error mapping
    if status.is_client_error() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read response body>".to_string());
        logger::warn!(
            "Hypersense returned 4xx: status = {:?}, body = {}",
            status,
            body
        );

        match status.as_u16() {
            404 => {
                // Not Found -> treat as invalid request URL
                return Err(errors::ApiErrorResponse::InvalidRequestUrl.into());
            }
            _ => {
                return Err(errors::ApiErrorResponse::InvalidRequestData { message: body }.into());
            }
        }
    }

    let data = response
        .json()
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error when deserializing response from Hypersense service")?;

    Ok(api::ApplicationResponse::Json(
        external_service_hypersense_api::ExternalFeeEstimateResponse::Hypersense { response: data },
    ))
}
