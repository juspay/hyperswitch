use common_utils::{
    ext_traits::Encode,
    request::{Method, Request, RequestBuilder},
};
use error_stack::{IntoReport, ResultExt};
use http::header;
use serde_json::json;

use crate::{
    connector,
    core::errors::{ApiErrorResponse, RouterResult},
    routes::AppState,
    types,
    types::api::{
        enums,
        verify_connector::{self as verify_connector_types, VerifyConnector},
    },
    utils::verify_connector as verify_connector_utils,
};

pub async fn generate_access_token(state: AppState) -> RouterResult<types::AccessToken> {
    let connector = enums::Connector::Paypal;
    let boxed_connector = types::api::ConnectorData::convert_connector(
        &state.conf.connectors,
        connector.to_string().as_str(),
    )?;
    let connector_auth = super::get_connector_auth(connector, &state.conf.connector_onboarding)?;

    connector::Paypal::get_access_token(
        &state,
        verify_connector_types::VerifyConnectorData {
            connector: *boxed_connector,
            connector_auth,
            card_details: verify_connector_utils::get_test_card_details(connector)?
                .ok_or(ApiErrorResponse::FlowNotSupported {
                    flow: "Connector onboarding".to_string(),
                    connector: connector.to_string(),
                })
                .into_report()?,
        },
    )
    .await?
    .ok_or(ApiErrorResponse::InternalServerError)
    .into_report()
    .attach_printable("Error occurred while retrieving access token")
}

pub fn build_paypal_post_request<T>(
    url: String,
    body: T,
    access_token: String,
) -> RouterResult<Request>
where
    T: serde::Serialize,
{
    let body = types::RequestBody::log_and_get_request_body(
        &json!(body),
        Encode::<serde_json::Value>::encode_to_string_of_json,
    )
    .change_context(ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to build request body")?;

    Ok(RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .header(
            header::AUTHORIZATION.to_string().as_str(),
            format!("Bearer {}", access_token).as_str(),
        )
        .header(
            header::CONTENT_TYPE.to_string().as_str(),
            "application/json",
        )
        .body(Some(body))
        .build())
}

pub fn build_paypal_get_request(url: String, access_token: String) -> RouterResult<Request> {
    Ok(RequestBuilder::new()
        .method(Method::Get)
        .url(&url)
        .attach_default_headers()
        .header(
            header::AUTHORIZATION.to_string().as_str(),
            format!("Bearer {}", access_token).as_str(),
        )
        .build())
}
