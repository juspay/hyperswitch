use error_stack::ResultExt;

use super::{VerifyConnector, VerifyConnectorData};
use crate::{
    connector,
    core::errors,
    routes::AppState,
    services,
    types::{self, api},
};

#[async_trait::async_trait]
impl VerifyConnector for connector::Paypal {
        /// Asynchronously retrieves an access token using the provided `AppState` and `VerifyConnectorData`.
    ///
    /// # Arguments
    ///
    /// * `state` - The `AppState` containing the application state.
    /// * `connector_data` - The `VerifyConnectorData` containing the connector data.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing either `Some` `AccessToken` or an `ApiErrorResponse`.
    ///
    /// # Errors
    ///
    /// An `ApiErrorResponse` is returned if there is an error in retrieving the access token.
    ///
    async fn get_access_token(
        state: &AppState,
        connector_data: VerifyConnectorData,
    ) -> errors::CustomResult<Option<types::AccessToken>, errors::ApiErrorResponse> {
        let token_data: types::AccessTokenRequestData =
            connector_data.connector_auth.clone().try_into()?;
        let router_data = connector_data.get_router_data(token_data, None);

        let request = connector_data
            .connector
            .build_request(&router_data, &state.conf.connectors)
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "Payment request cannot be built".to_string(),
            })?
            .ok_or(errors::ApiErrorResponse::InternalServerError)?;

        let response = services::call_connector_api(&state.to_owned(), request)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        match response {
            Ok(res) => Some(
                connector_data
                    .connector
                    .handle_response(&router_data, res)
                    .change_context(errors::ApiErrorResponse::InternalServerError)?
                    .response
                    .map_err(|_| errors::ApiErrorResponse::InternalServerError.into()),
            )
            .transpose(),
            Err(response_data) => {
                Self::handle_access_token_error_response::<
                    api::AccessTokenAuth,
                    types::AccessTokenRequestData,
                    types::AccessToken,
                >(connector_data.connector, response_data)
                .await
            }
        }
    }
}
