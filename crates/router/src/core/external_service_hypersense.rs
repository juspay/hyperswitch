use api_models::external_service_hypersense as external_service_hypersense_api;
use common_utils::{
    consts,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use external_services::http_client;

use router_env::logger;

use crate::{
    SessionState, core::{ errors::{self, RouterResponse}}, services::{api,
            authentication::{self}}
};


pub async fn get_hypersense_fee_estimate(
    state: SessionState,
    api_path: String,
    query_params: &str,
    json_payload: external_service_hypersense_api::ExternalFeeEstimatePayload,
    user: authentication::UserFromToken,
) -> RouterResponse<external_service_hypersense_api::ExternalFeeEstimateResponse> {
    // TODO: get base url from config
    let url = format!(
        "{}/fee-analysis/{}?{}",
        state.conf.hypersense.api_url, api_path, query_params
    );
    let combined = serde_json::json!({
        "payload": {
            "merchant_id": user.merchant_id,
            "payload": json_payload.payload,
        }
    });

    let request_builder = RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .set_body(RequestContent::Json(Box::new(combined)));

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

    let data = response
        .json()
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error when deserializing response from Hypersense service")?;

    Ok(api::ApplicationResponse::Json(
        external_service_hypersense_api::ExternalFeeEstimateResponse::Hypersense { response: data },
    ))
}
