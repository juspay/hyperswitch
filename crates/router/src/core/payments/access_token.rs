use std::fmt::Debug;

use common_utils::ext_traits::AsyncExt;
use error_stack::{IntoReport, ResultExt};

use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        payments,
    },
    routes::{metrics, AppState},
    services,
    types::{self, api as api_types, domain},
};

/// After we get the access token, check if there was an error and if the flow should proceed further
/// Returns bool
/// true - Everything is well, continue with the flow
/// false - There was an error, cannot proceed further
pub fn update_router_data_with_access_token_result<F, Req, Res>(
    add_access_token_result: &types::AddAccessTokenResult,
    router_data: &mut types::RouterData<F, Req, Res>,
    call_connector_action: &payments::CallConnectorAction,
) -> bool {
    // Update router data with access token or error only if it will be calling connector
    let should_update_router_data = matches!(
        (
            add_access_token_result.connector_supports_access_token,
            call_connector_action
        ),
        (true, payments::CallConnectorAction::Trigger)
    );

    if should_update_router_data {
        match add_access_token_result.access_token_result.as_ref() {
            Ok(access_token) => {
                router_data.access_token = access_token.clone();
                true
            }
            Err(connector_error) => {
                router_data.response = Err(connector_error.clone());
                false
            }
        }
    } else {
        true
    }
}

/// Adds an access token for the given merchant account and connector data. 
pub async fn add_access_token<
    F: Clone + 'static,
    Req: Debug + Clone + 'static,
    Res: Debug + Clone + 'static,
>(
    state: &AppState,
    connector: &api_types::ConnectorData,
    merchant_account: &domain::MerchantAccount,
    router_data: &types::RouterData<F, Req, Res>,
) -> RouterResult<types::AddAccessTokenResult> {
    if connector
        .connector_name
        .supports_access_token(router_data.payment_method)
    {
        let merchant_id = &merchant_account.merchant_id;
        let store = &*state.store;
        let old_access_token = store
            .get_access_token(merchant_id, connector.connector.id())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("DB error when accessing the access token")?;

        let res = match old_access_token {
            Some(access_token) => Ok(Some(access_token)),
            None => {
                let cloned_router_data = router_data.clone();
                let refresh_token_request_data = types::AccessTokenRequestData::try_from(
                    router_data.connector_auth_type.clone(),
                )
                .into_report()
                .attach_printable(
                    "Could not create access token request, invalid connector account credentials",
                )?;

                let refresh_token_response_data: Result<types::AccessToken, types::ErrorResponse> =
                    Err(types::ErrorResponse::default());
                let refresh_token_router_data = payments::helpers::router_data_type_conversion::<
                    _,
                    api_types::AccessTokenAuth,
                    _,
                    _,
                    _,
                    _,
                >(
                    cloned_router_data,
                    refresh_token_request_data,
                    refresh_token_response_data,
                );
                refresh_connector_auth(
                    state,
                    connector,
                    merchant_account,
                    &refresh_token_router_data,
                )
                .await?
                .async_map(|access_token| async {
                    //Store the access token in db
                    let store = &*state.store;
                    // This error should not be propagated, we don't want payments to fail once we have
                    // the access token, the next request will create new access token
                    let _ = store
                        .set_access_token(
                            merchant_id,
                            connector.connector.id(),
                            access_token.clone(),
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("DB error when setting the access token");
                    Some(access_token)
                })
                .await
            }
        };

        Ok(types::AddAccessTokenResult {
            access_token_result: res,
            connector_supports_access_token: true,
        })
    } else {
        Ok(types::AddAccessTokenResult {
            access_token_result: Err(types::ErrorResponse::default()),
            connector_supports_access_token: false,
        })
    }
}

/// Asynchronously refreshes the authentication token for the given connector and returns the updated access token.
pub async fn refresh_connector_auth(
    state: &AppState,
    connector: &api_types::ConnectorData,
    _merchant_account: &domain::MerchantAccount,
    router_data: &types::RouterData<
        api_types::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    >,
) -> RouterResult<Result<types::AccessToken, types::ErrorResponse>> {
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api_types::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > = connector.connector.get_connector_integration();

    let access_token_router_data_result = services::execute_connector_processing_step(
        state,
        connector_integration,
        router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await;

    let access_token_router_data = match access_token_router_data_result {
        Ok(router_data) => Ok(router_data.response),
        Err(connector_error) => {
            // If we receive a timeout error from the connector, then
            // the error has to be handled gracefully by updating the payment status to failed.
            // further payment flow will not be continued
            if connector_error.current_context().is_connector_timeout() {
                let error_response = types::ErrorResponse {
                    code: consts::REQUEST_TIMEOUT_ERROR_CODE.to_string(),
                    message: consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string(),
                    reason: Some(consts::REQUEST_TIMEOUT_ERROR_MESSAGE.to_string()),
                    status_code: 504,
                    attempt_status: None,
                    connector_transaction_id: None,
                };

                Ok(Err(error_response))
            } else {
                Err(connector_error
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Could not refresh access token"))
            }
        }
    }?;

    metrics::ACCESS_TOKEN_CREATION.add(
        &metrics::CONTEXT,
        1,
        &[metrics::request::add_attributes(
            "connector",
            connector.connector_name.to_string(),
        )],
    );
    Ok(access_token_router_data)
}
