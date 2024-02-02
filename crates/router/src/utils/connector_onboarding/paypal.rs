use common_utils::request::{Method, Request, RequestBuilder, RequestContent};
use error_stack::{IntoReport, ResultExt};
use http::header;

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

/// Asynchronously generates an access token for the specified state. 
/// This method retrieves the connector data for the Paypal connector from the application state, 
/// gets the connector authentication information, and then uses it to retrieve an access token 
/// for the Paypal connector. If successful, it returns the access token; otherwise, it returns 
/// an internal server error response.
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

/// This function takes a URL, a request body, and an access token, and returns a RouterResult containing a POST request to be sent to the specified URL. The request is constructed with the given URL, body, and access token, as well as default headers, authorization header with the access token, and a content type header set to "application/json".
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

/// Constructs a GET request for the given URL with the provided access token for authorization,
/// and returns a RouterResult containing the constructed Request.
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
