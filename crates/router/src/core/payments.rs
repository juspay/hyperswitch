pub mod access_token;
pub mod flows;
pub mod helpers;
pub mod operations;
pub mod transformers;

use std::{fmt::Debug, marker::PhantomData, time::Instant};

use common_utils::ext_traits::AsyncExt;
use error_stack::{IntoReport, ResultExt};
use futures::future::join_all;
use router_env::{instrument, tracing};
use time;

pub use self::operations::{
    PaymentCancel, PaymentCapture, PaymentConfirm, PaymentCreate, PaymentMethodValidate,
    PaymentResponse, PaymentSession, PaymentStatus, PaymentUpdate,
};
use self::{
    flows::{ConstructFlowSpecificData, Feature},
    operations::{payment_complete_authorize, BoxedOperation, Operation},
};
use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult},
        payment_methods::vault,
    },
    db::StorageInterface,
    logger, pii,
    routes::AppState,
    scheduler::utils as pt_utils,
    services,
    types::{
        self, api,
        storage::{self, enums as storage_enums},
    },
    utils::OptionExt,
};

#[instrument(skip_all)]
pub async fn payments_operation_core<F, Req, Op, FData>(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    operation: Op,
    req: Req,
    call_connector_action: CallConnectorAction,
) -> RouterResult<(PaymentData<F>, Req, Option<storage::Customer>)>
where
    F: Send + Clone,
    Op: Operation<F, Req> + Send + Sync,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, FData, types::PaymentsResponseData>,
    types::RouterData<F, FData, types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn types::api::Connector:
        services::api::ConnectorIntegration<F, FData, types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData>,
    FData: Send,
{
    let operation: BoxedOperation<'_, F, Req> = Box::new(operation);

    let (operation, validate_result) = operation
        .to_validate_request()?
        .validate_request(&req, &merchant_account)?;

    tracing::Span::current().record("payment_id", &format!("{:?}", validate_result.payment_id));

    let (operation, mut payment_data, customer_details) = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &validate_result.payment_id,
            &req,
            validate_result.mandate_type,
            &merchant_account,
        )
        .await?;

    let (operation, customer) = operation
        .to_domain()?
        .get_or_create_customer_details(
            &*state.store,
            &mut payment_data,
            customer_details,
            validate_result.merchant_id,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while fetching/creating customer")?;

    let (operation, payment_method_data) = operation
        .to_domain()?
        .make_pm_data(state, &mut payment_data, validate_result.storage_scheme)
        .await?;

    payment_data.payment_method_data = payment_method_data;

    let connector_details = operation
        .to_domain()?
        .get_connector(
            &merchant_account,
            state,
            &req,
            payment_data.payment_attempt.connector.as_ref(),
        )
        .await?;

    let connector = match should_call_connector(&operation, &payment_data) {
        true => Some(
            route_connector(
                state,
                &merchant_account,
                &mut payment_data,
                connector_details,
            )
            .await?,
        ),
        false => None,
    };

    let (operation, mut payment_data) = operation
        .to_update_tracker()?
        .update_trackers(
            &*state.store,
            &validate_result.payment_id,
            payment_data,
            customer.clone(),
            validate_result.storage_scheme,
        )
        .await?;

    operation
        .to_domain()?
        .add_task_to_process_tracker(state, &payment_data.payment_attempt)
        .await?;

    if let Some(connector_details) = connector {
        payment_data = match connector_details {
            api::ConnectorCallType::Single(connector) => {
                call_connector_service(
                    state,
                    &merchant_account,
                    &validate_result.payment_id,
                    connector,
                    &operation,
                    payment_data,
                    &customer,
                    call_connector_action,
                )
                .await?
            }
            api::ConnectorCallType::Multiple(connectors) => {
                call_multiple_connectors_service(
                    state,
                    &merchant_account,
                    connectors,
                    &operation,
                    payment_data,
                    &customer,
                )
                .await?
            }
            api::ConnectorCallType::Routing => {
                let connector = payment_data
                    .payment_attempt
                    .connector
                    .clone()
                    .get_required_value("connector")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("No connector selected for routing")?;

                let connector_data = api::ConnectorData::get_connector_by_name(
                    &state.conf.connectors,
                    &connector,
                    api::GetToken::Connector,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

                call_connector_service(
                    state,
                    &merchant_account,
                    &validate_result.payment_id,
                    connector_data,
                    &operation,
                    payment_data,
                    &customer,
                    call_connector_action,
                )
                .await?
            }
        };
        if payment_data.payment_intent.status != storage_enums::IntentStatus::RequiresCustomerAction
        {
            vault::Vault::delete_locker_payment_method_by_lookup_key(state, &payment_data.token)
                .await
        }
    }
    Ok((payment_data, req, customer))
}

#[allow(clippy::too_many_arguments)]
pub async fn payments_core<F, Res, Req, Op, FData>(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    operation: Op,
    req: Req,
    auth_flow: services::AuthFlow,
    call_connector_action: CallConnectorAction,
) -> RouterResponse<Res>
where
    F: Send + Clone,
    FData: Send,
    Op: Operation<F, Req> + Send + Sync + Clone,
    Req: Debug,
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
    let (payment_data, req, customer) = payments_operation_core(
        state,
        merchant_account,
        operation.clone(),
        req,
        call_connector_action,
    )
    .await?;

    Res::generate_response(
        Some(req),
        payment_data,
        customer,
        auth_flow,
        &state.conf.server,
        operation,
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
        merchant_account: storage::MerchantAccount,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
    ) -> RouterResponse<api::PaymentsResponse>;

    fn get_payment_action(&self) -> services::PaymentAction;

    #[allow(clippy::too_many_arguments)]
    async fn handle_payments_redirect_response(
        &self,
        state: &AppState,
        merchant_account: storage::MerchantAccount,
        req: PaymentsRedirectResponseData,
    ) -> RouterResponse<api::RedirectionResponse> {
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
            .call_payment_flow(state, merchant_account.clone(), req.clone(), flow_type)
            .await;

        let payments_response = match response? {
            services::ApplicationResponse::Json(response) => Ok(response),
            _ => Err(errors::ApiErrorResponse::InternalServerError)
                .into_report()
                .attach_printable("Failed to get the response in json"),
        }?;

        let result = helpers::get_handle_response_url(
            resource_id,
            &merchant_account,
            payments_response,
            connector,
        )
        .attach_printable("No redirection response")?;

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
        merchant_account: storage::MerchantAccount,
        req: PaymentsRedirectResponseData,
        connector_action: CallConnectorAction,
    ) -> RouterResponse<api::PaymentsResponse> {
        let payment_confirm_req = api::PaymentsRequest {
            payment_id: Some(req.resource_id.clone()),
            merchant_id: req.merchant_id.clone(),
            ..Default::default()
        };
        payments_core::<api::CompleteAuthorize, api::PaymentsResponse, _, _, _>(
            state,
            merchant_account,
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
}

#[derive(Clone, Debug)]
pub struct PaymentRedirectSync;

#[async_trait::async_trait]
impl PaymentRedirectFlow for PaymentRedirectSync {
    async fn call_payment_flow(
        &self,
        state: &AppState,
        merchant_account: storage::MerchantAccount,
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
        };
        payments_core::<api::PSync, api::PaymentsResponse, _, _, _>(
            state,
            merchant_account,
            PaymentStatus,
            payment_sync_req,
            services::api::AuthFlow::Merchant,
            connector_action,
        )
        .await
    }

    fn get_payment_action(&self) -> services::PaymentAction {
        services::PaymentAction::PSync
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn call_connector_service<F, Op, Req>(
    state: &AppState,
    merchant_account: &storage::MerchantAccount,
    payment_id: &api::PaymentIdType,
    connector: api::ConnectorData,
    _operation: &Op,
    payment_data: PaymentData<F>,
    customer: &Option<storage::Customer>,
    call_connector_action: CallConnectorAction,
) -> RouterResult<PaymentData<F>>
where
    Op: Debug + Sync,
    F: Send + Clone,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, Req, types::PaymentsResponseData>,
    types::RouterData<F, Req, types::PaymentsResponseData>: Feature<F, Req> + Send,

    // To construct connector flow specific api
    dyn api::Connector: services::api::ConnectorIntegration<F, Req, types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, Req>,
{
    let db = &*state.store;

    let stime_connector = Instant::now();

    let mut router_data = payment_data
        .construct_router_data(state, connector.connector.id(), merchant_account)
        .await?;

    let add_access_token_result = router_data
        .add_access_token(state, &connector, merchant_account)
        .await?;

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &call_connector_action,
    );

    let router_data_res = if !(add_access_token_result.connector_supports_access_token
        && router_data.access_token.is_none())
    {
        router_data
            .decide_flows(
                state,
                &connector,
                customer,
                call_connector_action,
                merchant_account,
            )
            .await
    } else {
        Ok(router_data)
    };

    let response = router_data_res
        .async_and_then(|response| async {
            let operation = helpers::response_operation::<F, Req>();
            let payment_data = operation
                .to_post_update_tracker()?
                .update_tracker(
                    db,
                    payment_id,
                    payment_data,
                    response,
                    merchant_account.storage_scheme,
                )
                .await?;
            Ok(payment_data)
        })
        .await?;

    let etime_connector = Instant::now();
    let duration_connector = etime_connector.saturating_duration_since(stime_connector);
    tracing::info!(duration = format!("Duration taken: {}", duration_connector.as_millis()));

    Ok(response)
}

pub async fn call_multiple_connectors_service<F, Op, Req>(
    state: &AppState,
    merchant_account: &storage::MerchantAccount,
    connectors: Vec<api::ConnectorData>,
    _operation: &Op,
    mut payment_data: PaymentData<F>,
    customer: &Option<storage::Customer>,
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

    for connector in connectors.iter() {
        let connector_id = connector.connector.id();
        let router_data = payment_data
            .construct_router_data(state, connector_id, merchant_account)
            .await?;

        let res = router_data.decide_flows(
            state,
            connector,
            customer,
            CallConnectorAction::Trigger,
            merchant_account,
        );

        join_handlers.push(res);
    }

    let result = join_all(join_handlers).await;

    for (connector_res, connector) in result.into_iter().zip(connectors) {
        let connector_name = connector.connector_name.to_string();
        match connector_res {
            Ok(connector_response) => {
                if let Ok(types::PaymentsResponseData::SessionResponse { session_token }) =
                    connector_response.response
                {
                    payment_data.sessions_token.push(session_token);
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

#[derive(Clone, Debug)]
pub enum CallConnectorAction {
    Trigger,
    Avoid,
    StatusUpdate(storage_enums::AttemptStatus),
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
    pub connector_response: storage::ConnectorResponse,
    pub amount: api::Amount,
    pub mandate_id: Option<api_models::payments::MandateIds>,
    pub currency: storage_enums::Currency,
    pub setup_mandate: Option<api::MandateData>,
    pub address: PaymentAddress,
    pub token: Option<String>,
    pub confirm: Option<bool>,
    pub force_sync: Option<bool>,
    pub payment_method_data: Option<api::PaymentMethodData>,
    pub refunds: Vec<storage::Refund>,
    pub sessions_token: Vec<api::SessionToken>,
    pub card_cvc: Option<pii::Secret<String>>,
    pub email: Option<masking::Secret<String, pii::Email>>,
    pub creds_identifier: Option<String>,
}

#[derive(Debug, Default)]
pub struct CustomerDetails {
    pub customer_id: Option<String>,
    pub name: Option<masking::Secret<String, masking::WithType>>,
    pub email: Option<masking::Secret<String, pii::Email>>,
    pub phone: Option<masking::Secret<String, masking::WithType>>,
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
                storage_enums::IntentStatus::Failed
                    | storage_enums::IntentStatus::Processing
                    | storage_enums::IntentStatus::Succeeded
                    | storage_enums::IntentStatus::RequiresCustomerAction
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
            )
        }
        "CompleteAuthorize" => true,
        "PaymentSession" => true,
        _ => false,
    }
}

#[cfg(feature = "olap")]
pub async fn list_payments(
    db: &dyn StorageInterface,
    merchant: storage::MerchantAccount,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<api::PaymentListResponse> {
    use futures::stream::StreamExt;

    use crate::types::transformers::ForeignFrom;

    helpers::validate_payment_list_request(&constraints)?;
    let merchant_id = &merchant.merchant_id;
    let payment_intents =
        helpers::filter_by_constraints(db, &constraints, merchant_id, merchant.storage_scheme)
            .await
            .map_err(|err| {
                errors::StorageErrorExt::to_not_found_response(
                    err,
                    errors::ApiErrorResponse::PaymentNotFound,
                )
            })?;

    let pi = futures::stream::iter(payment_intents)
        .filter_map(|pi| async {
            let pa = db
                .find_payment_attempt_by_payment_id_merchant_id(
                    &pi.payment_id,
                    merchant_id,
                    // since OLAP doesn't have KV. Force to get the data from PSQL.
                    storage_enums::MerchantStorageScheme::PostgresOnly,
                )
                .await
                .ok()?;
            Some((pi, pa))
        })
        .collect::<Vec<(storage::PaymentIntent, storage::PaymentAttempt)>>()
        .await;

    let data: Vec<api::PaymentsResponse> = pi.into_iter().map(ForeignFrom::foreign_from).collect();

    Ok(services::ApplicationResponse::Json(
        api::PaymentListResponse {
            size: data.len(),
            data,
        },
    ))
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

pub async fn route_connector<F>(
    state: &AppState,
    merchant_account: &storage::MerchantAccount,
    payment_data: &mut PaymentData<F>,
    connector_call_type: api::ConnectorCallType,
) -> RouterResult<api::ConnectorCallType>
where
    F: Send + Clone,
{
    match connector_call_type {
        api::ConnectorCallType::Single(connector) => {
            payment_data.payment_attempt.connector = Some(connector.connector_name.to_string());

            Ok(api::ConnectorCallType::Single(connector))
        }

        api::ConnectorCallType::Routing => {
            let routing_algorithm: api::RoutingAlgorithm = merchant_account
                .routing_algorithm
                .clone()
                .parse_value("RoutingAlgorithm")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Could not decode merchant routing rules")?;

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

            payment_data.payment_attempt.connector = Some(connector_name);

            Ok(api::ConnectorCallType::Single(connector_data))
        }

        call_type @ api::ConnectorCallType::Multiple(_) => Ok(call_type),
    }
}
