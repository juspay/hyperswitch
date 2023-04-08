use std::fmt::Debug;

use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;

use crate::{
    core::{
        errors::{self, RouterResult},
        payments,
    },
    routes::{metrics, AppState},
    services,
    types::{self, api as api_types, storage, transformers::ForeignInto},
};

/// This function replaces the request and response type of routerdata with the
/// request and response type passed
/// # Arguments
///
/// * `router_data` - original router data
/// * `request` - new request
/// * `response` - new response
pub fn router_data_type_conversion<F1, F2, Req1, Req2, Res1, Res2>(
    router_data: types::RouterData<F1, Req1, Res1>,
    request: Req2,
    response: Result<Res2, types::ErrorResponse>,
) -> types::RouterData<F2, Req2, Res2> {
    types::RouterData {
        flow: std::marker::PhantomData,
        request,
        response,
        merchant_id: router_data.merchant_id,
        address: router_data.address,
        amount_captured: router_data.amount_captured,
        auth_type: router_data.auth_type,
        connector: router_data.connector,
        connector_auth_type: router_data.connector_auth_type,
        connector_meta_data: router_data.connector_meta_data,
        description: router_data.description,
        payment_id: router_data.payment_id,
        payment_method: router_data.payment_method,
        payment_method_id: router_data.payment_method_id,
        return_url: router_data.return_url,
        status: router_data.status,
        attempt_id: router_data.attempt_id,
        access_token: router_data.access_token,
        session_token: router_data.session_token,
        reference_id: None,
    }
}

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

/// Supports access token and refresh token flows
///
/// Access Token Flow:
/// In access token flow, Once access token is generated it will be stored in redis.
/// TTL for the access token will be set from 'expires' field in AccessToken. After
/// expiration a new token has to be generated everytime.
///
/// Refresh Token Flow:
/// This flow also involves access token. But the main difference is, to get a new access
/// token refresh token has to be exchanged. Usually refresh token will have longer validity
/// than access token. In Refresh token flow we use the refresh token expiry as TTL of
/// AccessToken and this validity can be found in refresh_token_epires field of AccessToken.
/// Eventhough the refresh token has longer validity, once access token got expired a new
/// access token will be generated using AccessTokenAuth flow
pub async fn add_access_token<
    F: Clone + 'static,
    Req: Debug + Clone + 'static,
    Res: Debug + Clone + 'static,
>(
    state: &AppState,
    connector: &api_types::ConnectorData,
    merchant_account: &storage::MerchantAccount,
    router_data: &types::RouterData<F, Req, Res>,
) -> RouterResult<types::AddAccessTokenResult> {
    if connector
        .connector_name
        .supports_access_token(router_data.payment_method.foreign_into())
    {
        let merchant_id = &merchant_account.merchant_id;
        let store = &*state.store;
        let old_access_token = store
            .get_access_token(merchant_id, connector.connector.id())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("DB error when accessing the access token")?;

        let res = match is_new_access_token_required(old_access_token.as_ref()) {
            true => {
                let cloned_router_data = router_data.clone();
                let refresh_token_request_data = types::AccessTokenRequestData { old_access_token };
                let refresh_token_response_data: Result<types::AccessToken, types::ErrorResponse> =
                    Err(types::ErrorResponse::default());
                let refresh_token_router_data =
                    router_data_type_conversion::<_, api_types::AccessTokenAuth, _, _, _, _>(
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
            false => Ok(old_access_token),
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

pub fn is_new_access_token_required(old_access_token: Option<&types::AccessToken>) -> bool {
    match old_access_token {
        Some(access_token) => {
            // Access token is present
            match access_token.created_at {
                // If access_token is present along with created_at, then the current time should not exceed the expiration time
                Some(created_at) => {
                    let now = time::OffsetDateTime::now_utc().unix_timestamp();
                    now > (created_at + access_token.expires)
                }
                // If created_at is not present for the token, then the token can be cosidered valid.
                None => false,
            }
        }
        // Access token does not present, so new token has to be generated
        None => true,
    }
}

pub async fn refresh_connector_auth(
    state: &AppState,
    connector: &api_types::ConnectorData,
    _merchant_account: &storage::MerchantAccount,
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

    let access_token_router_data = services::execute_connector_processing_step(
        state,
        connector_integration,
        router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Could not refresh access token")?;
    metrics::ACCESS_TOKEN_CREATION.add(
        &metrics::CONTEXT,
        1,
        &[metrics::request::add_attributes(
            "connector",
            connector.connector_name.to_string(),
        )],
    );
    Ok(access_token_router_data.response)
}
