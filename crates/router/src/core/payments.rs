pub mod access_token;
pub mod customers;
pub mod flows;
pub mod helpers;
pub mod operations;
pub mod tokenization;
pub mod transformers;

use std::{collections::HashMap, fmt::Debug, marker::PhantomData, ops::Deref, time::Instant};

use api_models::payments::FrmMessage;
use common_utils::{ext_traits::AsyncExt, pii};
use diesel_models::ephemeral_key;
use data_models::mandates::MandateData;
use error_stack::{IntoReport, ResultExt};
use futures::future::join_all;
use masking::Secret;
use router_env::{instrument, tracing};
use time;

pub use self::operations::{
    PaymentCancel, PaymentCapture, PaymentConfirm, PaymentCreate, PaymentMethodValidate,
    PaymentResponse, PaymentSession, PaymentStatus, PaymentUpdate,
};
use self::{
    flows::{ConstructFlowSpecificData, Feature},
    helpers::authenticate_client_secret,
    operations::{payment_complete_authorize, BoxedOperation, Operation},
};
use super::errors::StorageErrorExt;
use crate::{
    configs::settings::PaymentMethodTypeTokenFilter,
    core::errors::{self, CustomResult, RouterResponse, RouterResult},
    db::StorageInterface,
    logger,
    routes::{metrics, payment_methods::ParentPaymentMethodToken, AppState},
    scheduler::{utils as pt_utils, workflows::payment_sync},
    services::{self, api::Authenticate},
    types::{
        self, api, domain,
        storage::{self, enums as storage_enums, ProcessTrackerExt},
    },
    utils::{add_connector_http_status_code_metrics, Encode, OptionExt, ValueExt},
};

#[instrument(skip_all, fields(payment_id, merchant_id))]
pub async fn payments_operation_core<F, Req, Op, FData>(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    call_connector_action: CallConnectorAction,
    auth_flow: services::AuthFlow,
) -> RouterResult<(PaymentData<F>, Req, Option<domain::Customer>, Option<u16>)>
where
    F: Send + Clone + Sync,
    Req: Authenticate,
    Op: Operation<F, Req> + Send + Sync,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, FData, types::PaymentsResponseData>,
    types::RouterData<F, FData, types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn types::api::Connector:
        services::api::ConnectorIntegration<F, FData, types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData>,
    FData: Send + Sync,
{
    let operation: BoxedOperation<'_, F, Req> = Box::new(operation);

    tracing::Span::current().record("merchant_id", merchant_account.merchant_id.as_str());
    let (operation, validate_result) = operation
        .to_validate_request()?
        .validate_request(&req, &merchant_account)?;

    tracing::Span::current().record("payment_id", &format!("{}", validate_result.payment_id));
    let (operation, mut payment_data, customer_details) = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &validate_result.payment_id,
            &req,
            validate_result.mandate_type.to_owned(),
            &merchant_account,
            &key_store,
            auth_flow,
        )
        .await?;

    authenticate_client_secret(
        req.get_client_secret(),
        &payment_data.payment_intent,
        merchant_account.intent_fulfillment_time,
    )?;

    let (operation, customer) = operation
        .to_domain()?
        .get_or_create_customer_details(
            &*state.store,
            &mut payment_data,
            customer_details,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("Failed while fetching/creating customer")?;

    let connector = get_connector_choice(
        &operation,
        state,
        &req,
        &merchant_account,
        &key_store,
        &mut payment_data,
    )
    .await?;

    let schedule_time = match &connector {
        Some(api::ConnectorCallType::Single(connector_data)) => {
            if should_add_task_to_process_tracker(&payment_data) {
                payment_sync::get_sync_process_schedule_time(
                    &*state.store,
                    connector_data.connector.id(),
                    &merchant_account.merchant_id,
                    0,
                )
                .await
                .into_report()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while getting process schedule time")?
            } else {
                None
            }
        }
        _ => None,
    };

    let (mut payment_data, tokenization_action) =
        get_connector_tokenization_action(state, &operation, payment_data, &validate_result)
            .await?;

    let updated_customer = call_create_connector_customer_if_required(
        state,
        &customer,
        &merchant_account,
        &key_store,
        &mut payment_data,
    )
    .await?;

    let mut connector_http_status_code = None;

    if let Some(connector_details) = connector {
        payment_data = match connector_details {
            api::ConnectorCallType::Single(connector) => {
                let router_data = call_connector_service(
                    state,
                    &merchant_account,
                    &key_store,
                    connector,
                    &operation,
                    &mut payment_data,
                    &customer,
                    call_connector_action,
                    tokenization_action,
                    updated_customer,
                    validate_result.requeue,
                    schedule_time,
                )
                .await?;

                let operation = Box::new(PaymentResponse);
                let db = &*state.store;
                connector_http_status_code = router_data.connector_http_status_code;
                //add connector http status code metrics
                add_connector_http_status_code_metrics(connector_http_status_code);
                operation
                    .to_post_update_tracker()?
                    .update_tracker(
                        db,
                        &validate_result.payment_id,
                        payment_data,
                        router_data,
                        merchant_account.storage_scheme,
                    )
                    .await?
            }

            api::ConnectorCallType::Multiple(connectors) => {
                call_multiple_connectors_service(
                    state,
                    &merchant_account,
                    &key_store,
                    connectors,
                    &operation,
                    payment_data,
                    &customer,
                )
                .await?
            }
        };
        payment_data
            .payment_attempt
            .payment_token
            .as_ref()
            .zip(payment_data.payment_attempt.payment_method)
            .map(ParentPaymentMethodToken::create_key_for_token)
            .async_map(|key_for_hyperswitch_token| async move {
                if key_for_hyperswitch_token
                    .should_delete_payment_method_token(payment_data.payment_intent.status)
                {
                    let _ = key_for_hyperswitch_token.delete(state).await;
                }
            })
            .await;
    } else {
        (_, payment_data) = operation
            .to_update_tracker()?
            .update_trackers(
                &*state.store,
                payment_data.clone(),
                customer.clone(),
                validate_result.storage_scheme,
                updated_customer,
                &key_store,
                None,
            )
            .await?;
    }

    Ok((payment_data, req, customer, connector_http_status_code))
}

#[allow(clippy::too_many_arguments)]
pub async fn payments_core<F, Res, Req, Op, FData>(
    state: &AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    operation: Op,
    req: Req,
    auth_flow: services::AuthFlow,
    call_connector_action: CallConnectorAction,
) -> RouterResponse<Res>
where
    F: Send + Clone + Sync,
    FData: Send + Sync,
    Op: Operation<F, Req> + Send + Sync + Clone,
    Req: Debug + Authenticate,
    Res: transformers::ToResponse<Req, PaymentData<F>, Op>,
    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, FData, types::PaymentsResponseData>,
    types::RouterData<F, FData, types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn types::api::Connector:
        services::api::ConnectorIntegration<F, FData, types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData>,
{
    let (payment_data, req, customer, connector_http_status_code) = payments_operation_core(
        state,
        merchant_account,
        key_store,
        operation.clone(),
        req,
        call_connector_action,
        auth_flow,
    )
    .await?;

    Res::generate_response(
        Some(req),
        payment_data,
        customer,
        auth_flow,
        &state.conf.server,
        operation,
        &state.conf.connector_request_reference_id_config,
        connector_http_status_code,
    )
}

fn is_start_pay<Op: Debug>(operation: &Op) -> bool {
    format!("{operation:?}").eq("PaymentStart")
}

#[derive(Clone, Debug)]
pub struct PaymentsRedirectResponseData {
    pub connector: Option<String>,
    pub param: Option<String>,
    pub merchant_id: Option<String>,
    pub json_payload: Option<serde_json::Value>,
    pub resource_id: api::PaymentIdType,
    pub force_sync: bool,
    pub creds_identifier: Option<String>,
}

#[async_trait::async_trait]
pub trait PaymentRedirectFlow: Sync {
    async fn call_payment_flow(
        &self,
        state: &AppState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
    ) -> RouterResponse<api::PaymentsResponse>;

    fn get_payment_action(&self) -> services::PaymentAction;

    fn generate_response(
        &self,
        payments_response: api_models::payments::PaymentsResponse,
        merchant_account: types::domain::MerchantAccount,
        payment_id: String,
        connector: String,
    ) -> RouterResult<api::RedirectionResponse>;

    #[allow(clippy::too_many_arguments)]
    async fn handle_payments_redirect_response(
        &self,
        state: &AppState,
        merchant_account: domain::MerchantAccount,
        key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
    ) -> RouterResponse<api::RedirectionResponse> {
        metrics::REDIRECTION_TRIGGERED.add(
            &metrics::CONTEXT,
            1,
            &[
                metrics::request::add_attributes(
                    "connector",
                    req.connector.to_owned().unwrap_or("null".to_string()),
                ),
                metrics::request::add_attributes(
                    "merchant_id",
                    merchant_account.merchant_id.to_owned(),
                ),
            ],
        );
        let connector = req.connector.clone().get_required_value("connector")?;

        let query_params = req.param.clone().get_required_value("param")?;

        let resource_id = api::PaymentIdTypeExt::get_payment_intent_id(&req.resource_id)
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_id",
            })?;

        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector,
            api::GetToken::Connector,
        )?;

        let flow_type = connector_data
            .connector
            .get_flow_type(
                &query_params,
                req.json_payload.clone(),
                self.get_payment_action(),
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to decide the response flow")?;

        let response = self
            .call_payment_flow(
                state,
                merchant_account.clone(),
                key_store,
                req.clone(),
                flow_type,
            )
            .await;

        let payments_response = match response? {
            services::ApplicationResponse::Json(response) => Ok(response),
            services::ApplicationResponse::JsonWithHeaders((response, _)) => Ok(response),
            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .into_report()
                .attach_printable("Failed to get the response in json"),
        }?;

        let result =
            self.generate_response(payments_response, merchant_account, resource_id, connector)?;

        Ok(services::ApplicationResponse::JsonForRedirection(result))
    }
}

#[derive(Clone, Debug)]
pub struct PaymentRedirectCompleteAuthorize;

#[async_trait::async_trait]
impl PaymentRedirectFlow for PaymentRedirectCompleteAuthorize {
    async fn call_payment_flow(
        &self,
        state: &AppState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
    ) -> RouterResponse<api::PaymentsResponse> {
        let payment_confirm_req = api::PaymentsRequest {
            payment_id: Some(req.resource_id.clone()),
            merchant_id: req.merchant_id.clone(),
            feature_metadata: Some(api_models::payments::FeatureMetadata {
                redirect_response: Some(api_models::payments::RedirectResponse {
                    param: req.param.map(Secret::new),
                    json_payload: Some(req.json_payload.unwrap_or(serde_json::json!({})).into()),
                }),
            }),
            ..Default::default()
        };
        payments_core::<api::CompleteAuthorize, api::PaymentsResponse, _, _, _>(
            state,
            merchant_account,
            merchant_key_store,
            payment_complete_authorize::CompleteAuthorize,
            payment_confirm_req,
            services::api::AuthFlow::Merchant,
            connector_action,
        )
        .await
    }

    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::CompleteAuthorize
    }

    fn generate_response(
        &self,
        payments_response: api_models::payments::PaymentsResponse,
        merchant_account: types::domain::MerchantAccount,
        payment_id: String,
        connector: String,
    ) -> RouterResult<api::RedirectionResponse> {
        // There might be multiple redirections needed for some flows
        // If the status is requires customer action, then send the startpay url again
        // The redirection data must have been provided and updated by the connector
        match payments_response.status {
            api_models::enums::IntentStatus::RequiresCustomerAction => {
                let startpay_url = payments_response
                    .next_action
                    .and_then(|next_action_data| match next_action_data {
                        api_models::payments::NextActionData::RedirectToUrl { redirect_to_url } => Some(redirect_to_url),
                        api_models::payments::NextActionData::DisplayBankTransferInformation { .. } => None,
                        api_models::payments::NextActionData::ThirdPartySdkSessionToken { .. } => None,
                        api_models::payments::NextActionData::QrCodeInformation{..} => None,
                        api_models::payments::NextActionData::DisplayVoucherInformation{ .. } => None,
                        api_models::payments::NextActionData::WaitScreenInformation{..} => None,
                    })
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .into_report()
                    .attach_printable(
                        "did not receive redirect to url when status is requires customer action",
                    )?;
                Ok(api::RedirectionResponse {
                    return_url: String::new(),
                    params: vec![],
                    return_url_with_query_params: startpay_url,
                    http_method: "GET".to_string(),
                    headers: vec![],
                })
            }
            // If the status is terminal status, then redirect to merchant return url to provide status
            api_models::enums::IntentStatus::Succeeded
            | api_models::enums::IntentStatus::Failed
            | api_models::enums::IntentStatus::Cancelled | api_models::enums::IntentStatus::RequiresCapture| api_models::enums::IntentStatus::Processing=> helpers::get_handle_response_url(
                payment_id,
                &merchant_account,
                payments_response,
                connector,
            ),
            _ => Err(errors::ApiErrorResponse::InternalServerError).into_report().attach_printable_lazy(|| format!("Could not proceed with payment as payment status {} cannot be handled during redirection",payments_response.status))?
        }
    }
}

#[derive(Clone, Debug)]
pub struct PaymentRedirectSync;

#[async_trait::async_trait]
impl PaymentRedirectFlow for PaymentRedirectSync {
    async fn call_payment_flow(
        &self,
        state: &AppState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
    ) -> RouterResponse<api::PaymentsResponse> {
        let payment_sync_req = api::PaymentsRetrieveRequest {
            resource_id: req.resource_id,
            merchant_id: req.merchant_id,
            param: req.param,
            force_sync: req.force_sync,
            connector: req.connector,
            merchant_connector_details: req.creds_identifier.map(|creds_id| {
                api::MerchantConnectorDetailsWrap {
                    creds_identifier: creds_id,
                    encoded_data: None,
                }
            }),
            client_secret: None,
            expand_attempts: None,
        };
        payments_core::<api::PSync, api::PaymentsResponse, _, _, _>(
            state,
            merchant_account,
            merchant_key_store,
            PaymentStatus,
            payment_sync_req,
            services::api::AuthFlow::Merchant,
            connector_action,
        )
        .await
    }

    fn generate_response(
        &self,
        payments_response: api_models::payments::PaymentsResponse,
        merchant_account: types::domain::MerchantAccount,
        payment_id: String,
        connector: String,
    ) -> RouterResult<api::RedirectionResponse> {
        helpers::get_handle_response_url(
            payment_id,
            &merchant_account,
            payments_response,
            connector,
        )
    }

    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::PSync
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn call_connector_service<F, RouterDReq, ApiRequest>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connector: api::ConnectorData,
    operation: &BoxedOperation<'_, F, ApiRequest>,
    payment_data: &mut PaymentData<F>,
    customer: &Option<domain::Customer>,
    call_connector_action: CallConnectorAction,
    tokenization_action: TokenizationAction,
    updated_customer: Option<storage::CustomerUpdate>,
    requeue: bool,
    schedule_time: Option<time::PrimitiveDateTime>,
) -> RouterResult<types::RouterData<F, RouterDReq, types::PaymentsResponseData>>
where
    F: Send + Clone + Sync,
    RouterDReq: Send + Sync,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, RouterDReq, types::PaymentsResponseData>,
    types::RouterData<F, RouterDReq, types::PaymentsResponseData>: Feature<F, RouterDReq> + Send,

    // To construct connector flow specific api
    dyn api::Connector:
        services::api::ConnectorIntegration<F, RouterDReq, types::PaymentsResponseData>,
{
    let stime_connector = Instant::now();

    let mut router_data = payment_data
        .construct_router_data(
            state,
            connector.connector.id(),
            merchant_account,
            key_store,
            customer,
        )
        .await?;

    let add_access_token_result = router_data
        .add_access_token(state, &connector, merchant_account)
        .await?;

    let mut should_continue_further = access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &call_connector_action,
    );

    let pm_token = router_data
        .add_payment_method_token(state, &connector, &tokenization_action)
        .await?;

    if let Some(payment_method_token) = pm_token {
        router_data.payment_method_token = Some(payment_method_token);
    };

    (router_data, should_continue_further) = complete_preprocessing_steps_if_required(
        state,
        &connector,
        payment_data,
        router_data,
        should_continue_further,
    )
    .await?;

    if let Ok(types::PaymentsResponseData::PreProcessingResponse {
        session_token: Some(session_token),
        ..
    }) = router_data.response.to_owned()
    {
        payment_data.sessions_token.push(session_token);
    };

    // In case of authorize flow, pre-task and post-tasks are being called in build request
    // if we do not want to proceed further, then the function will return Ok(None, false)
    let (connector_request, should_continue_further) = if should_continue_further {
        // Check if the actual flow specific request can be built with available data
        router_data
            .build_flow_specific_connector_request(state, &connector, call_connector_action.clone())
            .await?
    } else {
        (None, false)
    };

    if should_add_task_to_process_tracker(payment_data) {
        operation
            .to_domain()?
            .add_task_to_process_tracker(
                state,
                &payment_data.payment_attempt,
                requeue,
                schedule_time,
            )
            .await
            .map_err(|error| logger::error!(process_tracker_error=?error))
            .ok();
    }

    // Update the payment trackers just before calling the connector
    // Since the request is already built in the previous step,
    // there should be no error in request construction from hyperswitch end
    (_, *payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            &*state.store,
            payment_data.clone(),
            customer.clone(),
            merchant_account.storage_scheme,
            updated_customer,
            key_store,
            None,
        )
        .await?;

    let router_data_res = if should_continue_further {
        // The status of payment_attempt and intent will be updated in the previous step
        // update this in router_data.
        // This is added because few connector integrations do not update the status,
        // and rely on previous status set in router_data
        router_data.status = payment_data.payment_attempt.status;
        router_data
            .decide_flows(
                state,
                &connector,
                customer,
                call_connector_action,
                merchant_account,
                connector_request,
            )
            .await
    } else {
        Ok(router_data)
    };

    let etime_connector = Instant::now();
    let duration_connector = etime_connector.saturating_duration_since(stime_connector);
    tracing::info!(duration = format!("Duration taken: {}", duration_connector.as_millis()));

    router_data_res
}

pub async fn call_multiple_connectors_service<F, Op, Req>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    connectors: Vec<api::SessionConnectorData>,
    _operation: &Op,
    mut payment_data: PaymentData<F>,
    customer: &Option<domain::Customer>,
) -> RouterResult<PaymentData<F>>
where
    Op: Debug,
    F: Send + Clone,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, Req, types::PaymentsResponseData>,
    types::RouterData<F, Req, types::PaymentsResponseData>: Feature<F, Req>,

    // To construct connector flow specific api
    dyn api::Connector: services::api::ConnectorIntegration<F, Req, types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, Req>,
{
    let call_connectors_start_time = Instant::now();
    let mut join_handlers = Vec::with_capacity(connectors.len());

    for session_connector_data in connectors.iter() {
        let connector_id = session_connector_data.connector.connector.id();
        let router_data = payment_data
            .construct_router_data(state, connector_id, merchant_account, key_store, customer)
            .await?;

        let res = router_data.decide_flows(
            state,
            &session_connector_data.connector,
            customer,
            CallConnectorAction::Trigger,
            merchant_account,
            None,
        );

        join_handlers.push(res);
    }

    let result = join_all(join_handlers).await;

    for (connector_res, session_connector) in result.into_iter().zip(connectors) {
        let connector_name = session_connector.connector.connector_name.to_string();
        match connector_res {
            Ok(connector_response) => {
                if let Ok(types::PaymentsResponseData::SessionResponse { session_token, .. }) =
                    connector_response.response
                {
                    // If session token is NoSessionTokenReceived, it is not pushed into the sessions_token as there is no response or there can be some error
                    // In case of error, that error is already logged
                    if !matches!(
                        session_token,
                        api_models::payments::SessionToken::NoSessionTokenReceived,
                    ) {
                        payment_data.sessions_token.push(session_token);
                    }
                }
            }
            Err(connector_error) => {
                logger::error!(
                    "sessions_connector_error {} {:?}",
                    connector_name,
                    connector_error
                );
            }
        }
    }

    let call_connectors_end_time = Instant::now();
    let call_connectors_duration =
        call_connectors_end_time.saturating_duration_since(call_connectors_start_time);
    tracing::info!(duration = format!("Duration taken: {}", call_connectors_duration.as_millis()));

    Ok(payment_data)
}

pub async fn call_create_connector_customer_if_required<F, Req>(
    state: &AppState,
    customer: &Option<domain::Customer>,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
) -> RouterResult<Option<storage::CustomerUpdate>>
where
    F: Send + Clone + Sync,
    Req: Send + Sync,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, Req, types::PaymentsResponseData>,
    types::RouterData<F, Req, types::PaymentsResponseData>: Feature<F, Req> + Send,

    // To construct connector flow specific api
    dyn api::Connector: services::api::ConnectorIntegration<F, Req, types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, Req>,
{
    let connector_name = payment_data.payment_attempt.connector.clone();

    match connector_name {
        Some(connector_name) => {
            let connector = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &connector_name,
                api::GetToken::Connector,
            )?;

            let connector_label = helpers::get_connector_label(
                payment_data.payment_intent.business_country,
                &payment_data.payment_intent.business_label,
                payment_data.payment_attempt.business_sub_label.as_ref(),
                &connector_name,
            );

            let (should_call_connector, existing_connector_customer_id) =
                customers::should_call_connector_create_customer(
                    state,
                    &connector,
                    customer,
                    &connector_label,
                );

            if should_call_connector {
                // Create customer at connector and update the customer table to store this data
                let router_data = payment_data
                    .construct_router_data(
                        state,
                        connector.connector.id(),
                        merchant_account,
                        key_store,
                        customer,
                    )
                    .await?;

                let connector_customer_id = router_data
                    .create_connector_customer(state, &connector)
                    .await?;

                let customer_update = customers::update_connector_customer_in_customers(
                    &connector_label,
                    customer.as_ref(),
                    &connector_customer_id,
                )
                .await;

                payment_data.connector_customer_id = connector_customer_id;
                Ok(customer_update)
            } else {
                // Customer already created in previous calls use the same value, no need to update
                payment_data.connector_customer_id =
                    existing_connector_customer_id.map(ToOwned::to_owned);
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

async fn complete_preprocessing_steps_if_required<F, Req>(
    state: &AppState,
    connector: &api::ConnectorData,
    payment_data: &PaymentData<F>,
    mut router_data: types::RouterData<F, Req, types::PaymentsResponseData>,
    should_continue_payment: bool,
) -> RouterResult<(types::RouterData<F, Req, types::PaymentsResponseData>, bool)>
where
    F: Send + Clone + Sync,
    Req: Send + Sync,
    types::RouterData<F, Req, types::PaymentsResponseData>: Feature<F, Req> + Send,
    dyn api::Connector: services::api::ConnectorIntegration<F, Req, types::PaymentsResponseData>,
{
    //TODO: For ACH transfers, if preprocessing_step is not required for connectors encountered in future, add the check
    let router_data_and_should_continue_payment = match payment_data.payment_method_data.clone() {
        Some(api_models::payments::PaymentMethodData::BankTransfer(data)) => match data.deref() {
            api_models::payments::BankTransferData::AchBankTransfer { .. }
            | api_models::payments::BankTransferData::MultibancoBankTransfer { .. }
                if connector.connector_name == types::Connector::Stripe =>
            {
                if payment_data.payment_attempt.preprocessing_step_id.is_none() {
                    (
                        router_data.preprocessing_steps(state, connector).await?,
                        false,
                    )
                } else {
                    (router_data, should_continue_payment)
                }
            }
            _ => (router_data, should_continue_payment),
        },
        Some(api_models::payments::PaymentMethodData::Wallet(_)) => {
            if is_preprocessing_required_for_wallets(connector.connector_name.to_string()) {
                (
                    router_data.preprocessing_steps(state, connector).await?,
                    false,
                )
            } else {
                (router_data, should_continue_payment)
            }
        }
        Some(api_models::payments::PaymentMethodData::Card(_)) => {
            if connector.connector_name == types::Connector::Payme {
                router_data = router_data.preprocessing_steps(state, connector).await?;

                let is_error_in_response = router_data.response.is_err();
                // If is_error_in_response is true, should_continue_payment should be false, we should throw the error
                (router_data, !is_error_in_response)
            } else {
                (router_data, should_continue_payment)
            }
        }
        _ => (router_data, should_continue_payment),
    };

    Ok(router_data_and_should_continue_payment)
}

pub fn is_preprocessing_required_for_wallets(connector_name: String) -> bool {
    connector_name == *"trustpay" || connector_name == *"payme"
}

fn is_payment_method_tokenization_enabled_for_connector(
    state: &AppState,
    connector_name: &str,
    payment_method: &storage::enums::PaymentMethod,
    payment_method_type: &Option<storage::enums::PaymentMethodType>,
) -> RouterResult<bool> {
    let connector_tokenization_filter = state.conf.tokenization.0.get(connector_name);

    Ok(connector_tokenization_filter
        .map(|connector_filter| {
            connector_filter
                .payment_method
                .clone()
                .contains(payment_method)
                && is_payment_method_type_allowed_for_connector(
                    payment_method_type,
                    connector_filter.payment_method_type.clone(),
                )
        })
        .unwrap_or(false))
}

fn is_payment_method_type_allowed_for_connector(
    current_pm_type: &Option<storage::enums::PaymentMethodType>,
    pm_type_filter: Option<PaymentMethodTypeTokenFilter>,
) -> bool {
    match (*current_pm_type).zip(pm_type_filter) {
        Some((pm_type, type_filter)) => match type_filter {
            PaymentMethodTypeTokenFilter::AllAccepted => true,
            PaymentMethodTypeTokenFilter::EnableOnly(enabled) => enabled.contains(&pm_type),
            PaymentMethodTypeTokenFilter::DisableOnly(disabled) => !disabled.contains(&pm_type),
        },
        None => true, // Allow all types if payment_method_type is not present
    }
}

async fn decide_payment_method_tokenize_action(
    state: &AppState,
    connector_name: &str,
    payment_method: &storage::enums::PaymentMethod,
    pm_parent_token: Option<&String>,
    is_connector_tokenization_enabled: bool,
) -> RouterResult<TokenizationAction> {
    match pm_parent_token {
        None => {
            if is_connector_tokenization_enabled {
                Ok(TokenizationAction::TokenizeInConnectorAndRouter)
            } else {
                Ok(TokenizationAction::TokenizeInRouter)
            }
        }
        Some(token) => {
            let redis_conn = state
                .store
                .get_redis_conn()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get redis connection")?;

            let key = format!(
                "pm_token_{}_{}_{}",
                token.to_owned(),
                payment_method,
                connector_name
            );

            let connector_token_option = redis_conn
                .get_key::<Option<String>>(&key)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to fetch the token from redis")?;

            match connector_token_option {
                Some(connector_token) => Ok(TokenizationAction::ConnectorToken(connector_token)),
                None => {
                    if is_connector_tokenization_enabled {
                        Ok(TokenizationAction::TokenizeInConnector)
                    } else {
                        Ok(TokenizationAction::TokenizeInRouter)
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub enum TokenizationAction {
    TokenizeInRouter,
    TokenizeInConnector,
    TokenizeInConnectorAndRouter,
    ConnectorToken(String),
    SkipConnectorTokenization,
}

#[allow(clippy::too_many_arguments)]
pub async fn get_connector_tokenization_action<F, Req>(
    state: &AppState,
    operation: &BoxedOperation<'_, F, Req>,
    mut payment_data: PaymentData<F>,
    validate_result: &operations::ValidateResult<'_>,
) -> RouterResult<(PaymentData<F>, TokenizationAction)>
where
    F: Send + Clone,
{
    let connector = payment_data.payment_attempt.connector.to_owned();

    let is_mandate = payment_data
        .mandate_id
        .as_ref()
        .and_then(|inner| inner.mandate_reference_id.as_ref())
        .map(|mandate_reference| match mandate_reference {
            api_models::payments::MandateReferenceId::ConnectorMandateId(_) => true,
            api_models::payments::MandateReferenceId::NetworkMandateId(_) => false,
        })
        .unwrap_or(false);

    let payment_data_and_tokenization_action = match connector {
        Some(_) if is_mandate => (payment_data, TokenizationAction::SkipConnectorTokenization),
        Some(connector) if is_operation_confirm(&operation) => {
            let payment_method = &payment_data
                .payment_attempt
                .payment_method
                .get_required_value("payment_method")?;
            let payment_method_type = &payment_data.payment_attempt.payment_method_type;
            let is_connector_tokenization_enabled =
                is_payment_method_tokenization_enabled_for_connector(
                    state,
                    &connector,
                    payment_method,
                    payment_method_type,
                )?;

            let payment_method_action = decide_payment_method_tokenize_action(
                state,
                &connector,
                payment_method,
                payment_data.token.as_ref(),
                is_connector_tokenization_enabled,
            )
            .await?;

            let connector_tokenization_action = match payment_method_action {
                TokenizationAction::TokenizeInRouter => {
                    let (_operation, payment_method_data) = operation
                        .to_domain()?
                        .make_pm_data(state, &mut payment_data, validate_result.storage_scheme)
                        .await?;
                    payment_data.payment_method_data = payment_method_data;
                    TokenizationAction::SkipConnectorTokenization
                }

                TokenizationAction::TokenizeInConnector => TokenizationAction::TokenizeInConnector,
                TokenizationAction::TokenizeInConnectorAndRouter => {
                    let (_operation, payment_method_data) = operation
                        .to_domain()?
                        .make_pm_data(state, &mut payment_data, validate_result.storage_scheme)
                        .await?;

                    payment_data.payment_method_data = payment_method_data;
                    TokenizationAction::TokenizeInConnector
                }
                TokenizationAction::ConnectorToken(token) => {
                    payment_data.pm_token = Some(token);
                    TokenizationAction::SkipConnectorTokenization
                }
                TokenizationAction::SkipConnectorTokenization => {
                    TokenizationAction::SkipConnectorTokenization
                }
            };
            (payment_data, connector_tokenization_action)
        }
        _ => {
            let (_operation, payment_method_data) = operation
                .to_domain()?
                .make_pm_data(state, &mut payment_data, validate_result.storage_scheme)
                .await?;
            payment_data.payment_method_data = payment_method_data;
            (payment_data, TokenizationAction::SkipConnectorTokenization)
        }
    };

    Ok(payment_data_and_tokenization_action)
}

#[derive(Clone)]
pub enum CallConnectorAction {
    Trigger,
    Avoid,
    StatusUpdate {
        status: storage_enums::AttemptStatus,
        error_code: Option<String>,
        error_message: Option<String>,
    },
    HandleResponse(Vec<u8>),
}

#[derive(Clone, Default, Debug)]
pub struct PaymentAddress {
    pub shipping: Option<api::Address>,
    pub billing: Option<api::Address>,
}

#[derive(Clone)]
pub struct PaymentData<F>
where
    F: Clone,
{
    pub flow: PhantomData<F>,
    pub payment_intent: storage::PaymentIntent,
    pub payment_attempt: storage::PaymentAttempt,
    pub multiple_capture_data: Option<MultipleCaptureData>,
    pub connector_response: storage::ConnectorResponse,
    pub amount: api::Amount,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub mandate_connector: Option<String>,
    pub currency: storage_enums::Currency,
    pub setup_mandate: Option<MandateData>,
    pub address: PaymentAddress,
    pub token: Option<String>,
    pub confirm: Option<bool>,
    pub force_sync: Option<bool>,
    pub payment_method_data: Option<api::PaymentMethodData>,
    pub refunds: Vec<storage::Refund>,
    pub disputes: Vec<storage::Dispute>,
    pub attempts: Option<Vec<storage::PaymentAttempt>>,
    pub sessions_token: Vec<api::SessionToken>,
    pub card_cvc: Option<Secret<String>>,
    pub email: Option<pii::Email>,
    pub creds_identifier: Option<String>,
    pub pm_token: Option<String>,
    pub connector_customer_id: Option<String>,
    pub recurring_mandate_payment_data: Option<RecurringMandatePaymentData>,
    pub ephemeral_key: Option<ephemeral_key::EphemeralKey>,
    pub redirect_response: Option<api_models::payments::RedirectResponse>,
    pub frm_message: Option<FrmMessage>,
}

#[derive(Clone)]
pub struct MultipleCaptureData {
    previous_captures: Vec<storage::Capture>,
    current_capture: storage::Capture,
}

impl MultipleCaptureData {
    fn get_previously_blocked_amount(&self) -> i64 {
        self.previous_captures
            .iter()
            .fold(0, |accumulator, capture| {
                accumulator
                    + match capture.status {
                        storage_enums::CaptureStatus::Charged
                        | storage_enums::CaptureStatus::Pending => capture.amount,
                        storage_enums::CaptureStatus::Started
                        | storage_enums::CaptureStatus::Failed => 0,
                    }
            })
    }
    fn get_total_blocked_amount(&self) -> i64 {
        self.get_previously_blocked_amount()
            + match self.current_capture.status {
                api_models::enums::CaptureStatus::Charged
                | api_models::enums::CaptureStatus::Pending => self.current_capture.amount,
                api_models::enums::CaptureStatus::Failed
                | api_models::enums::CaptureStatus::Started => 0,
            }
    }
    fn get_previously_charged_amount(&self) -> i64 {
        self.previous_captures
            .iter()
            .fold(0, |accumulator, capture| {
                accumulator
                    + match capture.status {
                        storage_enums::CaptureStatus::Charged => capture.amount,
                        storage_enums::CaptureStatus::Pending
                        | storage_enums::CaptureStatus::Started
                        | storage_enums::CaptureStatus::Failed => 0,
                    }
            })
    }
    fn get_total_charged_amount(&self) -> i64 {
        self.get_previously_charged_amount()
            + match self.current_capture.status {
                storage_enums::CaptureStatus::Charged => self.current_capture.amount,
                storage_enums::CaptureStatus::Pending
                | storage_enums::CaptureStatus::Started
                | storage_enums::CaptureStatus::Failed => 0,
            }
    }
    fn get_captures_count(&self) -> RouterResult<i16> {
        i16::try_from(1 + self.previous_captures.len())
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while converting from usize to i16")
    }
    fn get_status_count(&self) -> HashMap<storage_enums::CaptureStatus, i16> {
        let mut hash_map: HashMap<storage_enums::CaptureStatus, i16> = HashMap::new();
        hash_map.insert(storage_enums::CaptureStatus::Charged, 0);
        hash_map.insert(storage_enums::CaptureStatus::Pending, 0);
        hash_map.insert(storage_enums::CaptureStatus::Started, 0);
        hash_map.insert(storage_enums::CaptureStatus::Failed, 0);
        hash_map
            .entry(self.current_capture.status)
            .and_modify(|count| *count += 1);
        self.previous_captures
            .iter()
            .fold(hash_map, |mut accumulator, capture| {
                let current_capture_status = capture.status;
                accumulator
                    .entry(current_capture_status)
                    .and_modify(|count| *count += 1);
                accumulator
            })
    }
    fn get_attempt_status(&self, authorized_amount: i64) -> storage_enums::AttemptStatus {
        let total_captured_amount = self.get_total_charged_amount();
        if authorized_amount == total_captured_amount {
            return storage_enums::AttemptStatus::Charged;
        }
        let status_count_map = self.get_status_count();
        if status_count_map.get(&storage_enums::CaptureStatus::Charged) > Some(&0) {
            storage_enums::AttemptStatus::PartialCharged
        } else {
            storage_enums::AttemptStatus::CaptureInitiated
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct RecurringMandatePaymentData {
    pub payment_method_type: Option<storage_enums::PaymentMethodType>, //required for making recurring payment using saved payment method through stripe
}

#[derive(Debug, Default, Clone)]
pub struct CustomerDetails {
    pub customer_id: Option<String>,
    pub name: Option<Secret<String, masking::WithType>>,
    pub email: Option<pii::Email>,
    pub phone: Option<Secret<String, masking::WithType>>,
    pub phone_country_code: Option<String>,
}

pub fn if_not_create_change_operation<'a, Op, F>(
    status: storage_enums::IntentStatus,
    confirm: Option<bool>,
    current: &'a Op,
) -> BoxedOperation<'_, F, api::PaymentsRequest>
where
    F: Send + Clone,
    Op: Operation<F, api::PaymentsRequest> + Send + Sync,
    &'a Op: Operation<F, api::PaymentsRequest>,
{
    if confirm.unwrap_or(false) {
        Box::new(PaymentConfirm)
    } else {
        match status {
            storage_enums::IntentStatus::RequiresConfirmation
            | storage_enums::IntentStatus::RequiresCustomerAction
            | storage_enums::IntentStatus::RequiresPaymentMethod => Box::new(current),
            _ => Box::new(&PaymentStatus),
        }
    }
}

pub fn is_confirm<'a, F: Clone + Send, R, Op>(
    operation: &'a Op,
    confirm: Option<bool>,
) -> BoxedOperation<'_, F, R>
where
    PaymentConfirm: Operation<F, R>,
    &'a PaymentConfirm: Operation<F, R>,
    Op: Operation<F, R> + Send + Sync,
    &'a Op: Operation<F, R>,
{
    if confirm.unwrap_or(false) {
        Box::new(&PaymentConfirm)
    } else {
        Box::new(operation)
    }
}

pub fn should_call_connector<Op: Debug, F: Clone>(
    operation: &Op,
    payment_data: &PaymentData<F>,
) -> bool {
    match format!("{operation:?}").as_str() {
        "PaymentConfirm" => true,
        "PaymentStart" => {
            !matches!(
                payment_data.payment_intent.status,
                storage_enums::IntentStatus::Failed | storage_enums::IntentStatus::Succeeded
            ) && payment_data
                .connector_response
                .authentication_data
                .is_none()
        }
        "PaymentStatus" => {
            matches!(
                payment_data.payment_intent.status,
                storage_enums::IntentStatus::Processing
                    | storage_enums::IntentStatus::RequiresCustomerAction
                    | storage_enums::IntentStatus::RequiresMerchantAction
                    | storage_enums::IntentStatus::RequiresCapture
                    | storage_enums::IntentStatus::PartiallyCaptured
            ) && payment_data.force_sync.unwrap_or(false)
        }
        "PaymentCancel" => matches!(
            payment_data.payment_intent.status,
            storage_enums::IntentStatus::RequiresCapture
        ),
        "PaymentCapture" => {
            matches!(
                payment_data.payment_intent.status,
                storage_enums::IntentStatus::RequiresCapture
                    | storage_enums::IntentStatus::PartiallyCaptured
            )
        }
        "CompleteAuthorize" => true,
        "PaymentSession" => true,
        _ => false,
    }
}

pub fn is_operation_confirm<Op: Debug>(operation: &Op) -> bool {
    matches!(format!("{operation:?}").as_str(), "PaymentConfirm")
}

#[cfg(feature = "olap")]
pub async fn list_payments(
    db: &dyn StorageInterface,
    merchant: domain::MerchantAccount,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<api::PaymentListResponse> {
    use data_models::errors::StorageError;

    use crate::types::transformers::ForeignFrom;

    helpers::validate_payment_list_request(&constraints)?;
    let merchant_id = &merchant.merchant_id;
    let payment_intents =
        helpers::filter_by_constraints(db, &constraints, merchant_id, merchant.storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let collected_futures = payment_intents.into_iter().map(|pi| {
        async {
            match db
                .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                    &pi.payment_id,
                    merchant_id,
                    &pi.active_attempt_id,
                    // since OLAP doesn't have KV. Force to get the data from PSQL.
                    storage_enums::MerchantStorageScheme::PostgresOnly,
                )
                .await
            {
                Ok(pa) => Some(Ok((pi, pa))),
                Err(error) => {
                    if matches!(error.current_context(), StorageError::ValueNotFound(_)) {
                        logger::warn!(
                            ?error,
                            "payment_attempts missing for payment_id : {}",
                            pi.payment_id,
                        );
                        return None;
                    }
                    Some(Err(error))
                }
            }
        }
    });

    //If any of the response are Err, we will get Result<Err(_)>
    let pi_pa_tuple_vec: Result<Vec<(storage::PaymentIntent, storage::PaymentAttempt)>, _> =
        join_all(collected_futures)
            .await
            .into_iter()
            .flatten() //Will ignore `None`, will only flatten 1 level
            .collect::<Result<Vec<(storage::PaymentIntent, storage::PaymentAttempt)>, _>>();
    //Will collect responses in same order async, leading to sorted responses

    //Converting Intent-Attempt array to Response if no error
    let data: Vec<api::PaymentsResponse> = pi_pa_tuple_vec
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .into_iter()
        .map(ForeignFrom::foreign_from)
        .collect();

    Ok(services::ApplicationResponse::Json(
        api::PaymentListResponse {
            size: data.len(),
            data,
        },
    ))
}
#[cfg(feature = "olap")]
pub async fn apply_filters_on_payments(
    db: &dyn StorageInterface,
    merchant: domain::MerchantAccount,
    constraints: api::PaymentListFilterConstraints,
) -> RouterResponse<api::PaymentListResponse> {
    use storage_impl::DataModelExt;

    use crate::types::transformers::ForeignFrom;

    let list: Vec<(storage::PaymentIntent, storage::PaymentAttempt)> = db
        .get_filtered_payment_intents_attempt(
            &merchant.merchant_id,
            &constraints.clone().into(),
            merchant.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?
        .into_iter()
        .map(|(pi, pa)| (pi, pa))
        .collect();

    let data: Vec<api::PaymentsResponse> =
        list.into_iter().map(ForeignFrom::foreign_from).collect();

    Ok(services::ApplicationResponse::Json(
        api::PaymentListResponse {
            size: data.len(),
            data,
        },
    ))
}

#[cfg(feature = "olap")]
pub async fn get_filters_for_payments(
    db: &dyn StorageInterface,
    merchant: domain::MerchantAccount,
    time_range: api::TimeRange,
) -> RouterResponse<api::PaymentListFilters> {
    use crate::types::transformers::ForeignFrom;

    let pi = db
        .filter_payment_intents_by_time_range_constraints(
            &merchant.merchant_id,
            &time_range,
            merchant.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let filters = db
        .get_filters_for_payments(
            pi.as_slice(),
            &merchant.merchant_id,
            // since OLAP doesn't have KV. Force to get the data from PSQL.
            storage_enums::MerchantStorageScheme::PostgresOnly,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    Ok(services::ApplicationResponse::Json(api::PaymentListFilters {
        connector: filters.connector,
        currency: filters.currency,
        status: filters.status,
        payment_method: filters.payment_method,
    }))
}

pub async fn add_process_sync_task(
    db: &dyn StorageInterface,
    payment_attempt: &storage::PaymentAttempt,
    schedule_time: time::PrimitiveDateTime,
) -> Result<(), errors::ProcessTrackerError> {
    let tracking_data = api::PaymentsRetrieveRequest {
        force_sync: true,
        merchant_id: Some(payment_attempt.merchant_id.clone()),
        resource_id: api::PaymentIdType::PaymentAttemptId(payment_attempt.attempt_id.clone()),
        ..Default::default()
    };
    let runner = "PAYMENTS_SYNC_WORKFLOW";
    let task = "PAYMENTS_SYNC";
    let process_tracker_id = pt_utils::get_process_tracker_id(
        runner,
        task,
        &payment_attempt.attempt_id,
        &payment_attempt.merchant_id,
    );
    let process_tracker_entry = <storage::ProcessTracker>::make_process_tracker_new(
        process_tracker_id,
        task,
        runner,
        tracking_data,
        schedule_time,
    )?;

    db.insert_process(process_tracker_entry).await?;
    Ok(())
}

pub async fn reset_process_sync_task(
    db: &dyn StorageInterface,
    payment_attempt: &storage::PaymentAttempt,
    schedule_time: time::PrimitiveDateTime,
) -> Result<(), errors::ProcessTrackerError> {
    let runner = "PAYMENTS_SYNC_WORKFLOW";
    let task = "PAYMENTS_SYNC";
    let process_tracker_id = pt_utils::get_process_tracker_id(
        runner,
        task,
        &payment_attempt.attempt_id,
        &payment_attempt.merchant_id,
    );
    let psync_process = db
        .find_process_by_id(&process_tracker_id)
        .await?
        .ok_or(errors::ProcessTrackerError::ProcessFetchingFailed)?;
    psync_process.reset(db, schedule_time).await?;
    Ok(())
}

pub fn update_straight_through_routing<F>(
    payment_data: &mut PaymentData<F>,
    request_straight_through: serde_json::Value,
) -> CustomResult<(), errors::ParsingError>
where
    F: Send + Clone,
{
    let _: api::RoutingAlgorithm = request_straight_through
        .clone()
        .parse_value("RoutingAlgorithm")
        .attach_printable("Invalid straight through routing rules format")?;

    payment_data.payment_attempt.straight_through_algorithm = Some(request_straight_through);

    Ok(())
}

pub async fn get_connector_choice<F, Req>(
    operation: &BoxedOperation<'_, F, Req>,
    state: &AppState,
    req: &Req,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_data: &mut PaymentData<F>,
) -> RouterResult<Option<api::ConnectorCallType>>
where
    F: Send + Clone,
{
    let connector_choice = operation
        .to_domain()?
        .get_connector(
            merchant_account,
            state,
            req,
            &payment_data.payment_intent,
            key_store,
        )
        .await?;

    let connector = if should_call_connector(operation, payment_data) {
        Some(match connector_choice {
            api::ConnectorChoice::SessionMultiple(session_connectors) => {
                api::ConnectorCallType::Multiple(session_connectors)
            }

            api::ConnectorChoice::StraightThrough(straight_through) => connector_selection(
                state,
                merchant_account,
                payment_data,
                Some(straight_through),
            )?,

            api::ConnectorChoice::Decide => {
                connector_selection(state, merchant_account, payment_data, None)?
            }
        })
    } else if let api::ConnectorChoice::StraightThrough(val) = connector_choice {
        update_straight_through_routing(payment_data, val)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update straight through routing algorithm")?;
        None
    } else {
        None
    };

    Ok(connector)
}

pub fn connector_selection<F>(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    payment_data: &mut PaymentData<F>,
    request_straight_through: Option<serde_json::Value>,
) -> RouterResult<api::ConnectorCallType>
where
    F: Send + Clone,
{
    if let Some(ref connector_name) = payment_data.payment_attempt.connector {
        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name,
            api::GetToken::Connector,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("invalid connector name received in payment attempt")?;

        return Ok(api::ConnectorCallType::Single(connector_data));
    }

    let mut routing_data = storage::RoutingData {
        routed_through: payment_data.payment_attempt.connector.clone(),
        algorithm: payment_data
            .payment_attempt
            .straight_through_algorithm
            .clone()
            .map(|val| val.parse_value("RoutingAlgorithm"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid straight through algorithm format in payment attempt")?,
    };

    let request_straight_through: Option<api::StraightThroughAlgorithm> = request_straight_through
        .map(|val| val.parse_value("StraightThroughAlgorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid straight through routing rules format")?;

    let decided_connector = decide_connector(
        state,
        merchant_account,
        request_straight_through,
        &mut routing_data,
    )?;

    let encoded_algorithm = routing_data
        .algorithm
        .map(|algo| Encode::<api::RoutingAlgorithm>::encode_to_value(&algo))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to serialize routing algorithm to serde value")?;

    payment_data.payment_attempt.connector = routing_data.routed_through;
    payment_data.payment_attempt.straight_through_algorithm = encoded_algorithm;

    Ok(decided_connector)
}

pub fn decide_connector(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    request_straight_through: Option<api::StraightThroughAlgorithm>,
    routing_data: &mut storage::RoutingData,
) -> RouterResult<api::ConnectorCallType> {
    if let Some(ref connector_name) = routing_data.routed_through {
        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            connector_name.as_str(),
            api::GetToken::Connector,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in 'routed_through'")?;

        return Ok(api::ConnectorCallType::Single(connector_data));
    }

    if let Some(routing_algorithm) = request_straight_through {
        let connector_name = match &routing_algorithm {
            api::StraightThroughAlgorithm::Single(conn) => conn.to_string(),
        };

        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector_name,
            api::GetToken::Connector,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in routing algorithm")?;

        routing_data.routed_through = Some(connector_name);
        routing_data.algorithm = Some(routing_algorithm);
        return Ok(api::ConnectorCallType::Single(connector_data));
    }

    if let Some(ref routing_algorithm) = routing_data.algorithm {
        let connector_name = match routing_algorithm {
            api::StraightThroughAlgorithm::Single(conn) => conn.to_string(),
        };

        let connector_data = api::ConnectorData::get_connector_by_name(
            &state.conf.connectors,
            &connector_name,
            api::GetToken::Connector,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in routing algorithm")?;

        routing_data.routed_through = Some(connector_name);
        return Ok(api::ConnectorCallType::Single(connector_data));
    }

    let routing_algorithm = merchant_account
        .routing_algorithm
        .clone()
        .get_required_value("RoutingAlgorithm")
        .change_context(errors::ApiErrorResponse::PreconditionFailed {
            message: "no routing algorithm has been configured".to_string(),
        })?
        .parse_value::<api::RoutingAlgorithm>("RoutingAlgorithm")
        .change_context(errors::ApiErrorResponse::InternalServerError) // Deserialization failed
        .attach_printable("Unable to deserialize merchant routing algorithm")?;

    let connector_name = match routing_algorithm {
        api::RoutingAlgorithm::Single(conn) => conn.to_string(),
    };

    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name,
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Routing algorithm gave invalid connector")?;

    routing_data.routed_through = Some(connector_name);

    Ok(api::ConnectorCallType::Single(connector_data))
}

pub fn should_add_task_to_process_tracker<F: Clone>(payment_data: &PaymentData<F>) -> bool {
    let connector = payment_data.payment_attempt.connector.as_deref();

    !matches!(
        (payment_data.payment_attempt.payment_method, connector),
        (
            Some(storage_enums::PaymentMethod::BankTransfer),
            Some("stripe")
        )
    )
}
