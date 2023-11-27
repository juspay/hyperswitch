use crate::{connector, core::errors, types::api, utils::OptionExt};
use api_models::enums::Connector;
use error_stack::{IntoReport, ResultExt};

use crate::types::api::verify_connector::{self as types, VerifyConnector};
use crate::utils::verify_connector as utils;
use crate::{services, AppState};

pub async fn verify_connector_credentials(
    state: AppState,
    req: api::MerchantConnectorCreate,
) -> errors::RouterResponse<()> {
    let boxed_connector = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &req.connector_name.to_string(),
        api::GetToken::Connector,
        None,
    )
    .change_context(errors::ApiErrorResponse::IncorrectConnectorNameGiven)?;
    let connector_auth = req
        .connector_account_details
        .clone()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "connector_account_details".to_string(),
            expected_format: "auth_type and api_key".to_string(),
        })?;

    match req.connector_name {
        Connector::Stripe => {
            connector::Stripe::verify(
                &state,
                types::VerifyConnectorData {
                    connector: *boxed_connector.connector,
                    connector_auth,
                    card_details: utils::get_test_card_details(req.connector_name)?,
                },
            )
            .await
        }
        Connector::Paypal => connector::Paypal::get_access_token(
            &state,
            types::VerifyConnectorData {
                connector: *boxed_connector.connector,
                connector_auth,
                card_details: utils::get_test_card_details(req.connector_name)?,
            },
        )
        .await
        .map(|_| services::ApplicationResponse::StatusOk),
        _ => Err(errors::ApiErrorResponse::NotImplemented {
            message: errors::api_error_response::NotImplementedMessage::Reason(format!(
                "Verification for {}",
                req.connector_name
            )),
        })
        .into_report(),
    }
}
