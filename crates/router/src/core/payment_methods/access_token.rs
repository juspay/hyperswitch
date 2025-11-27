use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use hyperswitch_domain_models::types::VaultRouterData;

use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        payments,
    },
    routes::{metrics, SessionState},
    services,
    types::{
        self,
        api::{self as api_types, ConnectorCommon},
        domain,
    },
};

pub async fn create_access_token<F: Clone + 'static>(
    state: &SessionState,
    connector_data: &api_types::ConnectorData,
    merchant_account: &domain::MerchantAccount,
    router_data: &mut VaultRouterData<F>,
) -> RouterResult<()> {
    let connector_access_token =
        add_access_token_for_external_vault(state, connector_data, merchant_account, router_data)
            .await?;

    if connector_access_token.connector_supports_access_token {
        match connector_access_token.access_token_result {
            Ok(access_token) => {
                router_data.access_token = access_token;
            }
            Err(connector_error) => {
                router_data.response = Err(connector_error);
            }
        }
    }

    Ok(())
}

pub async fn add_access_token_for_external_vault<F: Clone + 'static>(
    state: &SessionState,
    connector: &api_types::ConnectorData,
    merchant_account: &domain::MerchantAccount,
    router_data: &VaultRouterData<F>,
) -> RouterResult<types::AddAccessTokenResult> {
    if connector
        .connector_name
        .supports_access_token_for_external_vault()
    {
        let merchant_id = merchant_account.get_id();
        let store = &*state.store;
        let key = common_utils::access_token::get_default_access_token_key(
            merchant_id,
            connector.connector.id(),
        );
        let old_access_token = store
            .get_access_token(key)
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
                refresh_connector_auth(state, connector, &refresh_token_router_data)
                    .await?
                    .async_map(|access_token| async {
                        //Store the access token in db
                        let store = &*state.store;
                        // This error should not be propagated, we don't want payments to fail once we have
                        // the access token, the next request will create new access token
                        let key = common_utils::access_token::get_default_access_token_key(
                            merchant_id,
                            connector.connector.id(),
                        );
                        let _ = store
                            .set_access_token(key, access_token.clone())
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

pub async fn refresh_connector_auth(
    state: &SessionState,
    connector: &api_types::ConnectorData,
    router_data: &types::RouterData<
        api_types::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    >,
) -> RouterResult<Result<types::AccessToken, types::ErrorResponse>> {
    let connector_integration: services::BoxedAccessTokenConnectorIntegrationInterface<
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
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
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
        1,
        router_env::metric_attributes!(("connector", connector.connector_name.to_string())),
    );
    Ok(access_token_router_data)
}
