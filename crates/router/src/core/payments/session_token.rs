use std::fmt::Debug;

use common_enums::enums;
use error_stack::{report, ResultExt};
use hyperswitch_interfaces::api::{gateway, ConnectorSpecifications};

use crate::{
    core::{
        errors,
        errors::utils::ConnectorErrorExt,
        payments::{gateway::context as gateway_context, RouterResult},
    },
    logger, routes, services, types,
    types::{api as api_types, transformers::ForeignFrom},
};

pub(crate) async fn add_session_token_if_needed<F: Clone, Req: Debug + Clone>(
    router_data: &types::RouterData<F, Req, types::PaymentsResponseData>,
    state: &routes::SessionState,
    connector: &api_types::ConnectorData,
    gateway_context: &gateway_context::RouterGatewayContext,
) -> RouterResult<Option<String>>
where
    types::AuthorizeSessionTokenData:
        for<'a> ForeignFrom<&'a types::RouterData<F, Req, types::PaymentsResponseData>>,
{
    if connector
        .connector
        .is_authorize_session_token_call_required()
    {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api_types::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let authorize_session_token_router_data =
            &types::PaymentsAuthorizeSessionTokenRouterData::foreign_from((
                router_data,
                types::AuthorizeSessionTokenData::foreign_from(router_data),
            ));
        let resp = gateway::execute_payment_gateway(
            state,
            connector_integration,
            authorize_session_token_router_data,
            enums::CallConnectorAction::Trigger,
            None,
            None,
            gateway_context.clone(),
        )
        .await
        .to_payment_failed_response()?;
        let session_token_respone = resp
            .response
            .map_err(|error| {
                logger::error!(session_token_create_error = error.reason);
                report!(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Faied to perform session token call".to_string()
                })
            })
            .attach_printable(format!(
                "Failed to create session token for connector: {:?}",
                connector.connector
            ))?;
        let session_token = match session_token_respone {
            types::PaymentsResponseData::SessionTokenResponse { session_token } => {
                Ok(session_token)
            }
            _ => Err(report!(errors::ApiErrorResponse::InternalServerError)).attach_printable(
                "Found Unexpected Response for Authorize Session Token Response from Connector",
            ),
        }?;
        Ok(Some(session_token))
    } else {
        Ok(None)
    }
}
