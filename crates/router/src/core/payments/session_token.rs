use std::convert::From;

use error_stack::{IntoReport, ResultExt};

use super::helpers;
use crate::{
    core::{errors, payments},
    routes::AppState,
    services,
    types::{self, api as api_types},
};

pub async fn session_token<'rd, F, Req, Res>(
    router_data: &'rd types::RouterData<F, Req, Res>,
    state: &AppState,
    connector: &api_types::ConnectorData,
) -> errors::RouterResult<types::SessionTokenResult>
where
    F: Clone + 'static,
    Req: Clone + 'static,
    Res: Clone + 'static,
    types::AuthorizeSessionTokenData: From<&'rd types::RouterData<F, Req, Res>>,
{
    let session_token_request_data = types::AuthorizeSessionTokenData::from(router_data);

    let session_token_response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
        Err(types::ErrorResponse::default());

    let session_token_router_data =
        helpers::router_data_type_conversion::<_, api_types::AuthorizeSessionToken, _, _, _, _>(
            router_data.clone(),
            session_token_request_data,
            session_token_response_data,
        );

    let connector_integration: services::BoxedConnectorIntegration<
        'static,
        api_types::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
    > = connector.connector.get_connector_integration();

    let resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        &session_token_router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    match resp.response {
        Ok(types::PaymentsResponseData::SessionTokenResponse { session_token }) => {
            Ok(types::SessionTokenResult { session_token })
        }
        _ => Err(errors::ConnectorError::ResponseHandlingFailed)
            .into_report()
            .attach_printable("Invalid mapping of seesion token response")
            .change_context(errors::ApiErrorResponse::InternalServerError),
    }
}
