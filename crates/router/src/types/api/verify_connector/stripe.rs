use error_stack::{IntoReport, ResultExt};
use router_env::env;

use super::VerifyConnector;
use crate::{
    connector,
    core::errors,
    services::{self, ConnectorIntegration},
    types,
};

#[async_trait::async_trait]
impl VerifyConnector for connector::Stripe {
        /// Handles the error response from a payment API, converting it into a router response.
    ///
    /// # Arguments
    ///
    /// * `connector` - A reference to a type that implements the Connector trait and Sync trait.
    /// * `error_response` - The error response received from the payment API.
    ///
    /// # Returns
    ///
    /// A Result containing either a router response with a success status or an error response with a message.
    ///
    /// # Generic Types
    ///
    /// * `F` - The type of the first input parameter for the ConnectorIntegration trait.
    /// * `R1` - The type of the first output parameter for the ConnectorIntegration trait.
    /// * `R2` - The type of the second output parameter for the ConnectorIntegration trait.
    ///
    /// # Constraints
    ///
    /// The type that implements the Connector trait and Sync trait must also implement the ConnectorIntegration trait with the specified generic types.
    ///
    /// # Remarks
    ///
    /// This method handles specific error scenarios from the payment API, such as a "card_declined" error when using a production key with a test card. It returns an Ok response in the production environment for this scenario, and an error response for other scenarios.
    ///
    async fn handle_payment_error_response<F, R1, R2>(
        connector: &(dyn types::api::Connector + Sync),
        error_response: types::Response,
    ) -> errors::RouterResponse<()>
    where
        dyn types::api::Connector + Sync: ConnectorIntegration<F, R1, R2>,
    {
        let error = connector
            .get_error_response(error_response)
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        match (env::which(), error.code.as_str()) {
            // In situations where an attempt is made to process a payment using a
            // Stripe production key along with a test card (which verify_connector is using),
            // Stripe will respond with a "card_declined" error. In production,
            // when this scenario occurs we will send back an "Ok" response.
            (env::Env::Production, "card_declined") => Ok(services::ApplicationResponse::StatusOk),
            _ => Err(errors::ApiErrorResponse::InvalidRequestData {
                message: error.reason.unwrap_or(error.message),
            })
            .into_report(),
        }
    }
}
