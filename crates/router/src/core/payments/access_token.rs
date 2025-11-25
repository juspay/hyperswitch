use std::fmt::Debug;

use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use hyperswitch_interfaces::api::{ConnectorAccessTokenSuffix, ConnectorSpecifications};

use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        payments,
    },
    routes::{metrics, SessionState},
    services::{self, logger},
    types::{self, api as api_types, domain},
};

/// Get cached access token for UCS flows - only reads from cache, never generates
pub async fn get_cached_access_token_for_ucs(
    state: &SessionState,
    connector: &api_types::ConnectorData,
    platform: &domain::Platform,
    payment_method: common_enums::PaymentMethod,
    creds_identifier: Option<&str>,
) -> RouterResult<Option<types::AccessToken>> {
    if connector
        .connector_name
        .supports_access_token(payment_method)
    {
        let merchant_id = platform.get_processor().get_account().get_id();
        let store = &*state.store;

        let merchant_connector_id_or_connector_name = connector
            .merchant_connector_id
            .clone()
            .map(|mca_id| mca_id.get_string_repr().to_string())
            .or(creds_identifier.map(|id| id.to_string()))
            .unwrap_or(connector.connector_name.to_string());

        let key = common_utils::access_token::get_default_access_token_key(
            merchant_id,
            merchant_connector_id_or_connector_name,
        );

        let cached_access_token = store
            .get_access_token(key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("DB error when accessing the access token")?;

        if let Some(access_token) = cached_access_token {
            router_env::logger::info!(
                "Cached access token found for UCS flow - merchant_id: {:?}, connector: {} with expiry of: {} seconds",
                platform.get_processor().get_account().get_id(),
                connector.connector_name,
                access_token.expires
            );
            metrics::ACCESS_TOKEN_CACHE_HIT.add(
                1,
                router_env::metric_attributes!(("connector", connector.connector_name.to_string())),
            );
            Ok(Some(access_token))
        } else {
            router_env::logger::info!(
                "No cached access token found for UCS flow - UCS will generate internally - merchant_id: {:?}, connector: {}",
                platform.get_processor().get_account().get_id(),
                connector.connector_name
            );
            metrics::ACCESS_TOKEN_CACHE_MISS.add(
                1,
                router_env::metric_attributes!(("connector", connector.connector_name.to_string())),
            );
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

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
                router_data.access_token.clone_from(access_token);
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

pub async fn add_access_token<
    F: Clone + 'static,
    Req: Debug + Clone + 'static,
    Res: Debug + Clone + 'static,
>(
    state: &SessionState,
    connector: &api_types::ConnectorData,
    router_data: &types::RouterData<F, Req, Res>,
    creds_identifier: Option<&str>,
) -> RouterResult<types::AddAccessTokenResult> {
    if connector
        .connector_name
        .supports_access_token(router_data.payment_method)
    {
        let merchant_id = &router_data.merchant_id;
        let store = &*state.store;

        // `merchant_connector_id` may not be present in the below cases
        // - when straight through routing is used without passing the `merchant_connector_id`
        // - when creds identifier is passed
        //
        // In these cases fallback to `connector_name`.
        // We cannot use multiple merchant connector account in these cases
        let merchant_connector_id_or_connector_name = connector
            .merchant_connector_id
            .clone()
            .map(|mca_id| mca_id.get_string_repr().to_string())
            .or(creds_identifier.map(|id| id.to_string()))
            .unwrap_or(connector.connector_name.to_string());

        let key = connector
            .connector
            .get_access_token_key(router_data, merchant_connector_id_or_connector_name.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!(
                "Failed to get access token key for connector: {:?}",
                connector.connector_name
            ))?;

        let old_access_token = store
            .get_access_token(key.clone())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("DB error when accessing the access token")?;

        let res = match old_access_token {
            Some(access_token) => {
                router_env::logger::debug!(
                    "Access token found in redis for merchant_id: {:?}, payment_id: {:?}, connector: {} which has expiry of: {} seconds",
                    merchant_id,
                    router_data.payment_id,
                    connector.connector_name,
                    access_token.expires
                );
                metrics::ACCESS_TOKEN_CACHE_HIT.add(
                    1,
                    router_env::metric_attributes!((
                        "connector",
                        connector.connector_name.to_string()
                    )),
                );
                Ok(Some(access_token))
            }
            None => {
                metrics::ACCESS_TOKEN_CACHE_MISS.add(
                    1,
                    router_env::metric_attributes!((
                        "connector",
                        connector.connector_name.to_string()
                    )),
                );

                let authentication_token =
                    execute_authentication_token(state, connector, router_data).await?;

                let cloned_router_data = router_data.clone();

                let refresh_token_request_data = types::AccessTokenRequestData::try_from((
                    router_data.connector_auth_type.clone(),
                    authentication_token,
                ))
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
                    .async_map(|access_token| async move {
                        let store = &*state.store;

                        // The expiry should be adjusted for network delays from the connector
                        // The access token might not have been expired when request is sent
                        // But once it reaches the connector, it might expire because of the network delay
                        // Subtract few seconds from the expiry in order to account for these network delays
                        // This will reduce the expiry time by `REDUCE_ACCESS_TOKEN_EXPIRY_TIME` seconds
                        let modified_access_token_with_expiry = types::AccessToken {
                            expires: access_token
                                .expires
                                .saturating_sub(consts::REDUCE_ACCESS_TOKEN_EXPIRY_TIME.into()),
                            ..access_token
                        };

                        logger::debug!(
                            access_token_expiry_after_modification =
                                modified_access_token_with_expiry.expires
                        );

                        if let Err(access_token_set_error) = store
                            .set_access_token(
                                key.clone(),
                                modified_access_token_with_expiry.clone(),
                            )
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("DB error when setting the access token")
                        {
                            // If we are not able to set the access token in redis, the error should just be logged and proceed with the payment
                            // Payments should not fail, once the access token is successfully created
                            // The next request will create new access token, if required
                            logger::error!(access_token_set_error=?access_token_set_error);
                        }
                        Some(modified_access_token_with_expiry)
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

pub async fn execute_authentication_token<
    F: Clone + 'static,
    Req: Debug + Clone + 'static,
    Res: Debug + Clone + 'static,
>(
    state: &SessionState,
    connector: &api_types::ConnectorData,
    router_data: &types::RouterData<F, Req, Res>,
) -> RouterResult<Option<types::AccessTokenAuthenticationResponse>> {
    let should_create_authentication_token = connector
        .connector
        .authentication_token_for_token_creation();

    if !should_create_authentication_token {
        return Ok(None);
    }

    let authentication_token_request_data = types::AccessTokenAuthenticationRequestData::try_from(
        router_data.connector_auth_type.clone(),
    )
    .attach_printable(
        "Could not create authentication token request, invalid connector account credentials",
    )?;

    let authentication_token_response_data: Result<
        types::AccessTokenAuthenticationResponse,
        types::ErrorResponse,
    > = Err(types::ErrorResponse::default());

    let auth_token_router_data = payments::helpers::router_data_type_conversion::<
        _,
        api_types::AccessTokenAuthentication,
        _,
        _,
        _,
        _,
    >(
        router_data.clone(),
        authentication_token_request_data,
        authentication_token_response_data,
    );

    let connector_integration: services::BoxedAuthenticationTokenConnectorIntegrationInterface<
        api_types::AccessTokenAuthentication,
        types::AccessTokenAuthenticationRequestData,
        types::AccessTokenAuthenticationResponse,
    > = connector.connector.get_connector_integration();

    let auth_token_router_data_result = services::execute_connector_processing_step(
        state,
        connector_integration,
        &auth_token_router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await;

    let auth_token_result = match auth_token_router_data_result {
        Ok(router_data) => router_data.response,
        Err(connector_error) => {
            // Handle timeout errors
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
                Err(error_response)
            } else {
                return Err(connector_error
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Could not get authentication token"));
            }
        }
    };

    let authentication_token = auth_token_result
        .map_err(|_error| errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get authentication token")?;

    Ok(Some(authentication_token))
}
