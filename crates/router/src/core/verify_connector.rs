use api_models::{enums::Connector, verify_connector::VerifyConnectorRequest};
use error_stack::ResultExt;

use crate::{
    connector,
    core::errors,
    services,
    types::{
        api,
        api::verify_connector::{self as types, VerifyConnector},
    },
    utils::verify_connector as utils,
    AppState,
};

pub async fn verify_connector_credentials(
    state: AppState,
    req: VerifyConnectorRequest,
) -> errors::RouterResponse<()> {
    let boxed_connector = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &req.connector_name.to_string(),
        api::GetToken::Connector,
        None,
    )
    .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)?;

    let card_details = utils::get_test_card_details(req.connector_name)?.ok_or(
        errors::ApiErrorResponse::FlowNotSupported {
            flow: "Verify credentials".to_string(),
            connector: req.connector_name.to_string(),
        },
    )?;

    match req.connector_name {
        Connector::Stripe => {
            connector::Stripe::verify(
                &state,
                types::VerifyConnectorData {
                    connector: *boxed_connector.connector,
                    connector_auth: req.connector_account_details.into(),
                    card_details,
                },
            )
            .await
        }
        Connector::Paypal => connector::Paypal::get_access_token(
            &state,
            types::VerifyConnectorData {
                connector: *boxed_connector.connector,
                connector_auth: req.connector_account_details.into(),
                card_details,
            },
        )
        .await
        .map(|_| services::ApplicationResponse::StatusOk),
        _ => Err(errors::ApiErrorResponse::FlowNotSupported {
            flow: "Verify credentials".to_string(),
            connector: req.connector_name.to_string(),
        }),
    }
}
