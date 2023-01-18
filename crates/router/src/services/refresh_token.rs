use common_utils::ext_traits::AsyncExt;
use error_stack::ResultExt;
use std::fmt::Debug;

use crate::{
    core::{
        errors::{self, RouterResult},
        payments,
    },
    routes::AppState,
    services,
    types::{self, api as api_types, storage},
};

pub fn connector_supports_access_token(connector: &api_types::ConnectorData) -> bool {
    match connector.connector_name {
        api_models::enums::Connector::Globalpay => true,
        api_models::enums::Connector::Stripe => false,
        _ => false,
    }
}

pub fn router_data_type_conversion<F1, F2, Req1, Req2, Res1, Res2>(
    router_data: types::RouterData<F1, Req1, Res1>,
    request: Req2,
    response: Result<Res2, types::ErrorResponse>,
) -> (
    types::RouterData<F2, Req2, Res2>,
    Req1,
    Result<Res1, types::ErrorResponse>,
) {
    (
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
            router_return_url: router_data.router_return_url,
            payment_id: router_data.payment_id,
            payment_method: router_data.payment_method,
            payment_method_id: router_data.payment_method_id,
            return_url: router_data.return_url,
            status: router_data.status,
            attempt_id: router_data.attempt_id,
            access_token: router_data.access_token,
        },
        router_data.request,
        router_data.response,
    )
}

pub async fn update_connector_auth<
    F: Clone + 'static,
    Req: Debug + Clone + 'static,
    Res: Debug + Clone + 'static,
>(
    state: &AppState,
    connector: &api_types::ConnectorData,
    merchant_account: &storage::MerchantAccount,
    router_data: &types::RouterData<F, Req, Res>,
) -> RouterResult<(
    Result<Option<types::AccessToken>, types::ErrorResponse>,
    bool,
)> {
    if connector_supports_access_token(connector) {
        let merchant_id = &merchant_account.merchant_id;
        let db = &*state.store;
        let old_access_token = db
            .get_access_token(merchant_id, connector.connector.id())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("DB error when accessing the access token")?;

        let res = match old_access_token {
            Some(access_token) => Ok(Some(access_token)),
            None => {
                let cloned_router_data = router_data.clone();
                let refresh_token_request_data =
                    types::RefreshTokenRequestData::from(router_data.connector_auth_type.clone());
                let refresh_token_response_data: Result<types::AccessToken, types::ErrorResponse> =
                    Err(types::ErrorResponse::default());
                let (refresh_token_router_data, _previous_request, _previous_response) =
                    router_data_type_conversion::<_, api_types::UpdateAuth, _, _, _, _>(
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
                    let db = &*state.store;
                    // This error should not be propogated, we don't want payments to fail once we have
                    // the access token
                    let _ = db
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

        Ok((res, true))
    } else {
        Ok((Err(types::ErrorResponse::default()), false))
    }
}

pub async fn refresh_connector_auth(
    state: &AppState,
    connector: &api_types::ConnectorData,
    _merchant_account: &storage::MerchantAccount,
    router_data: &types::RouterData<
        api_types::UpdateAuth,
        types::RefreshTokenRequestData,
        types::AccessToken,
    >,
) -> RouterResult<Result<types::AccessToken, types::ErrorResponse>> {
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api_types::UpdateAuth,
        types::RefreshTokenRequestData,
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

    Ok(access_token_router_data.response)
}
