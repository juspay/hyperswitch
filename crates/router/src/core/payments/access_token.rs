use common_utils::ext_traits::{AsyncExt, ValueExt};
use error_stack::{IntoReport, ResultExt};

use crate::{
    core::{
        errors::{self, RouterResult},
        payments,
    },
    db,
    routes::AppState,
    scheduler::workflows::{AccessTokenRefresh, ProcessTrackerWorkflow},
    services,
    types::{self, api as api_types, storage},
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProcessTrackerAccessTokenData {
    // Required to construct the request
    pub access_token_request: types::AccessTokenRequestData,

    // Fields required to construct router data
    pub merchant_id: String,
    pub connector: String,
    pub payment_id: String,
    pub attempt_id: String,
    pub payment_method: storage_models::enums::PaymentMethod,
}

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
        router_return_url: router_data.router_return_url,
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
) {
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
            Ok(access_token) => router_data.access_token = access_token.clone(),
            Err(connector_error) => router_data.response = Err(connector_error.clone()),
        }
    }
}

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
    if connector.connector_name.supports_access_token() {
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
                let refresh_token_router_data =
                    router_data_type_conversion::<_, api_types::AccessTokenAuth, _, _, _, _>(
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
                        let _ = store
                            .set_access_token(
                                merchant_id,
                                connector.connector.id(),
                                access_token.clone(),
                            )
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("DB error when setting the access token");

                        // Scheduler to get a new access token 60 seconds before it expires
                        let time_untill_refresh = access_token.expires.saturating_sub(60);

                        // let next_schedule_time = common_utils::date_time::now()
                        //     .saturating_add(time::Duration::seconds(time_untill_refresh));

                        let next_schedule_time = common_utils::date_time::now()
                            .saturating_add(time::Duration::seconds(30));

                        let _ = add_access_token_refresh_task(
                            store,
                            next_schedule_time,
                            &refresh_token_router_data,
                        )
                        .await;

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

pub async fn add_access_token_refresh_task<Flow, Response>(
    db: &dyn db::StorageInterface,
    schedule_time: time::PrimitiveDateTime,
    router_data: &types::RouterData<Flow, types::AccessTokenRequestData, Response>,
) -> Result<(), errors::ProcessTrackerError> {
    let tracking_data = ProcessTrackerAccessTokenData {
        access_token_request: router_data.request.clone(),
        merchant_id: router_data.merchant_id.clone(),
        connector: router_data.connector.clone(),
        payment_id: router_data.payment_id.clone(),
        attempt_id: router_data.attempt_id.clone(),
        payment_method: router_data.payment_method,
    };

    let runner = "ACCESS_TOKEN_REFRESH";
    let task = "ACCESS_TOKEN_REFRESH";

    let process_tracker_id = format!("{}_{}", task, router_data.connector);

    let process_tracker_entry =
        <storage::ProcessTracker as storage::ProcessTrackerExt>::make_process_tracker_new(
            process_tracker_id,
            task,
            runner,
            tracking_data,
            schedule_time,
        )?;

    db.insert_process(process_tracker_entry).await?;
    Ok(())
}

pub async fn refresh_connector_auth(
    state: &AppState,
    connector: &api_types::ConnectorData,
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

    Ok(access_token_router_data.response)
}

fn construct_access_token_router_data(
    tracking_data: &ProcessTrackerAccessTokenData,
    merchant_connector_account: &storage::MerchantConnectorAccount,
) -> RouterResult<
    types::RouterData<
        api_types::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    >,
> {
    let connector_auth_type: types::ConnectorAuthType = merchant_connector_account
        .connector_account_details
        .clone()
        .parse_value("ConnectorAuthType")
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "connector_account_details",
        })?;

    let access_token_request =
        types::AccessTokenRequestData::try_from(connector_auth_type.clone())?;

    Ok(types::RouterData {
        flow: std::marker::PhantomData,
        merchant_id: tracking_data.merchant_id.clone(),
        connector: tracking_data.connector.clone(),
        payment_id: tracking_data.payment_id.clone(),
        attempt_id: tracking_data.attempt_id.clone(),
        status: storage_models::enums::AttemptStatus::Pending,
        payment_method: tracking_data.payment_method,
        connector_auth_type,
        description: None,
        return_url: None,
        router_return_url: None,
        address: types::PaymentAddress::default(),
        auth_type: storage_models::enums::AuthenticationType::default(),
        connector_meta_data: None,
        amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        request: access_token_request,
        response: Err(types::ErrorResponse::default()),
        payment_method_id: None,
    })
}

#[async_trait::async_trait]
impl ProcessTrackerWorkflow for AccessTokenRefresh {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data: ProcessTrackerAccessTokenData =
            process.tracking_data.parse_value("AccessTokenData")?;
        let db: &dyn db::StorageInterface = &*state.store;

        let merchant_connector_account = db
            .find_merchant_connector_account_by_merchant_id_connector(
                &tracking_data.merchant_id,
                &tracking_data.connector,
            )
            .await?;

        let access_token_router_data =
            construct_access_token_router_data(&tracking_data, &merchant_connector_account)?;

        let connector = types::api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &tracking_data.connector,
            types::api::GetToken::Connector,
        )?;

        let _new_access_token =
            refresh_connector_auth(state, &connector, &access_token_router_data).await?;

        //TODO: update status of process tracker
        //TODO: schedule task for next refresh

        Ok(())
    }
}
