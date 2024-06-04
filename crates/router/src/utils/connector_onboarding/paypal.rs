use common_utils::request::{Method, Request, RequestBuilder, RequestContent};
use error_stack::ResultExt;
use http::header;

use crate::{
    connector,
    core::errors::{ApiErrorResponse, RouterResult},
    routes::SessionState,
    types,
    types::api::{
        enums,
        verify_connector::{self as verify_connector_types, VerifyConnector},
    },
    utils::verify_connector as verify_connector_utils,
};

pub async fn generate_access_token(state: SessionState) -> RouterResult<types::AccessToken> {
    let connector = enums::Connector::Paypal;
    let boxed_connector = types::api::ConnectorData::convert_connector(
        &state.conf.connectors,
        connector.to_string().as_str(),
    )?;
    let connector_auth =
        super::get_connector_auth(connector, state.conf.connector_onboarding.get_inner())?;

    connector::Paypal::get_access_token(
        &state,
        verify_connector_types::VerifyConnectorData {
            connector: *boxed_connector,
            connector_auth,
            card_details: verify_connector_utils::get_test_card_details(connector)?.ok_or(
                ApiErrorResponse::FlowNotSupported {
                    flow: "Connector onboarding".to_string(),
                    connector: connector.to_string(),
                },
            )?,
        },
    )
    .await?
    .ok_or(ApiErrorResponse::InternalServerError)
    .attach_printable("Error occurred while retrieving access token")
}

pub fn build_paypal_post_request<T>(
    url: String,
    body: T,
    access_token: String,
) -> RouterResult<Request>
where
    T: serde::Serialize + Send + 'static,
{
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
        .set_body(RequestContent::Json(Box::new(body)))
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
