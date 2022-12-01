pub mod flows;
pub mod helpers;
pub mod operations;
pub mod transformers;

use std::{fmt::Debug, marker::PhantomData, time::Instant};

use error_stack::{IntoReport, ResultExt};
use router_env::{tracing, tracing::instrument};
use time;

pub use self::operations::{
    PaymentCancel, PaymentCapture, PaymentConfirm, PaymentCreate, PaymentResponse, PaymentStatus,
    PaymentUpdate,
};
use self::{
    flows::{ConstructFlowSpecificData, Feature},
    helpers::{filter_by_constraints, validate_payment_list_request},
    operations::{BoxedOperation, Operation},
};
use super::errors::StorageErrorExt;
use crate::{
    core::{
        errors::{self, RouterResponse, RouterResult},
        payments,
        payments::transformers as payments_transformers,
    },
    db::Db,
    pii::Email,
    routes::AppState,
    scheduler::utils as pt_utils,
    services,
    types::{
        self,
        api::{self, PaymentsResponse, PaymentsRetrieveRequest},
        storage::{self, enums},
    },
    utils::{self, OptionExt},
};

#[instrument(skip_all)]
pub async fn payments_operation_core<F, Req, Op, FData>(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    operation: Op,
    req: Req,
    call_connector_action: CallConnectorAction,
) -> RouterResult<(PaymentData<F>, Req, Option<api::CustomerResponse>)>
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
{
    let connector = api::ConnectorData::construct(&state.conf.connectors, &merchant_account)
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let operation: BoxedOperation<F, Req> = Box::new(operation);

    let (operation, merchant_id, payment_id, mandate_type) = operation
        .to_validate_request()?
        .validate_request(&req, &merchant_account)?;

    tracing::Span::current().record("payment_id", &format!("{:?}", payment_id));

    let (operation, mut payment_data, customer_details) = operation
        .to_get_tracker()?
        .get_trackers(
            state,
            &payment_id,
            merchant_id,
            connector.connector_name,
            &req,
            mandate_type,
        )
        .await?;

    let (operation, customer) = operation
        .to_domain()?
        .get_or_create_customer_details(
            &state.store,
            &mut payment_data,
            customer_details,
            merchant_id,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let (operation, payment_method_data) = operation
        .to_domain()?
        .make_pm_data(
            state,
            payment_data.payment_attempt.payment_method,
            &payment_data.payment_attempt.txn_id,
            &payment_data.payment_attempt,
            &payment_data.payment_method_data,
            &payment_data.token,
        )
        .await?;
    payment_data.payment_method_data = payment_method_data;

    let (operation, mut payment_data) = operation
        .to_update_tracker()?
        .update_trackers(&state.store, &payment_id, payment_data, customer.clone())
        .await?;

    operation
        .to_domain()?
        .add_task_to_process_tracker(state, &payment_data.payment_attempt)
        .await?;

    if should_call_connector(&operation, &payment_data) {
        payment_data = call_connector_service(
            state,
            &merchant_account,
            &payment_id,
            connector,
            &operation,
            payment_data,
            &customer,
            call_connector_action,
        )
        .await?;
    }
    Ok((payment_data, req, customer))
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn payments_core<F, Req, Op, FData>(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    operation: Op,
    req: Req,
    auth_flow: services::AuthFlow,
    call_connector_action: CallConnectorAction,
) -> RouterResponse<api::PaymentsResponse>
where
    F: Send + Clone,
    Op: Operation<F, Req> + Send + Sync + Clone,
    Req: Debug,

    // To create connector flow specific interface data
    PaymentData<F>: ConstructFlowSpecificData<F, FData, types::PaymentsResponseData>,
    types::RouterData<F, FData, types::PaymentsResponseData>: Feature<F, FData>,

    // To construct connector flow specific api
    dyn types::api::Connector:
        services::api::ConnectorIntegration<F, FData, types::PaymentsResponseData>,

    // To perform router related operation for PaymentResponse
    PaymentResponse: Operation<F, FData>,

    // To create merchant response
    api::PaymentsResponse: From<Req>,
{
    let (payment_data, req, customer) = payments_operation_core(
        state,
        merchant_account,
        operation.clone(),
        req,
        call_connector_action,
    )
    .await?;

    payments_transformers::payments_to_payments_response(
        Some(req),
        payment_data.payment_attempt,
        payment_data.payment_intent,
        payment_data.refunds,
        payment_data.mandate_id,
        payment_data.payment_method_data,
        customer,
        auth_flow,
        payment_data.address,
        &state.conf.server,
        payment_data.connector_response.authentication_data,
        operation,
    )
}

fn is_start_pay<Op: Debug>(operation: &Op) -> bool {
    format!("{:?}", operation).eq("PaymentStart")
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_payments_redirect_response<'a, F>(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: PaymentsRetrieveRequest,
) -> RouterResponse<api::RedirectionResponse>
where
    F: Send + Clone + 'a,
{
    let connector = req.connector.clone().get_required_value("connector")?;

    let query_params = req.param.clone().get_required_value("param")?;

    let resource_id = req.resource_id.get_payment_intent_id().change_context(
        errors::ApiErrorResponse::MissingRequiredField {
            field_name: "payment_id".to_string(),
        },
    )?;

    let connector_data =
        api::ConnectorData::get_connector_by_name(&state.conf.connectors, &connector)?;

    let flow_type = connector_data
        .connector
        .get_flow_type(&query_params)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to decide the response flow")?;

    let response = payments_response_for_redirection_flows(
        state,
        merchant_account.clone(),
        req.clone(),
        flow_type,
    )
    .await;

    let payments_response =
        match response.change_context(errors::ApiErrorResponse::NotImplemented)? {
            services::BachResponse::Json(response) => Ok(response),
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

    Ok(services::BachResponse::JsonForRedirection(result))
}

pub async fn payments_response_for_redirection_flows<'a>(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    req: PaymentsRetrieveRequest,
    flow_type: CallConnectorAction,
) -> RouterResponse<PaymentsResponse> {
    payments_core::<api::PSync, _, _, _>(
        state,
        merchant_account,
        payments::PaymentStatus,
        req,
        services::api::AuthFlow::Merchant,
        flow_type,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn call_connector_service<F, Op, Req>(
    state: &AppState,
    merchant_account: &storage::MerchantAccount,
    payment_id: &api::PaymentIdType,
    connector: api::ConnectorData,
    _operation: &Op,
    payment_data: PaymentData<F>,
    customer: &Option<api::CustomerResponse>,
    call_connector_action: CallConnectorAction,
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
    let db = &state.store;

    let stime_connector = Instant::now();

    let router_data = payment_data
        .construct_r_d(state, connector.connector.id(), merchant_account)
        .await?;
    let (res, payment_data) = router_data
        .decide_flows(
            state,
            connector,
            customer,
            payment_data,
            call_connector_action,
        )
        .await;
    let response = helpers::amap(res, |response| async {
        let operation = helpers::response_operation::<F, Req>();
        let payment_data = operation
            .to_post_update_tracker()?
            .update_tracker(db, payment_id, payment_data, Some(response))
            .await?;
        Ok(payment_data)
    })
    .await?;

    let etime_connector = Instant::now();
    let duration_connector = etime_connector.saturating_duration_since(stime_connector);
    tracing::info!(duration = format!("Duration taken: {}", duration_connector.as_millis()));

    Ok(response)
}

pub enum CallConnectorAction {
    Trigger,
    Avoid,
    StatusUpdate(enums::AttemptStatus),
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
    pub amount: i32,
    pub currency: enums::Currency,
    pub mandate_id: Option<String>,
    pub setup_mandate: Option<api::MandateData>,
    pub address: PaymentAddress,
    pub token: Option<String>,
    pub confirm: Option<bool>,
    pub force_sync: Option<bool>,
    pub payment_method_data: Option<api::PaymentMethod>,
    pub refunds: Vec<storage::Refund>,
}

#[derive(Debug)]
pub struct CustomerDetails {
    pub customer_id: Option<String>,
    pub name: Option<masking::Secret<String, masking::WithType>>,
    pub email: Option<masking::Secret<String, Email>>,
    pub phone: Option<masking::Secret<String, masking::WithType>>,
    pub phone_country_code: Option<String>,
}

pub fn if_not_create_change_operation<'a, Op, F>(
    is_update: bool,
    status: enums::IntentStatus,
    current: &'a Op,
) -> BoxedOperation<F, api::PaymentsRequest>
where
    F: Send + Clone,
    Op: Operation<F, api::PaymentsRequest> + Send + Sync,
    &'a Op: Operation<F, api::PaymentsRequest>,
{
    match status {
        enums::IntentStatus::RequiresConfirmation
        | enums::IntentStatus::RequiresCustomerAction
        | enums::IntentStatus::RequiresPaymentMethod => {
            if is_update {
                Box::new(&PaymentUpdate)
            } else {
                Box::new(current)
            }
        }
        _ => Box::new(&PaymentStatus),
    }
}

pub fn is_confirm<'a, F: Clone + Send, R, Op>(
    operation: &'a Op,
    confirm: Option<bool>,
) -> BoxedOperation<F, R>
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
    match format!("{:?}", operation).as_str() {
        "PaymentConfirm" => {
            payment_data
                .payment_attempt
                .authentication_type
                .unwrap_or_default()
                == enums::AuthenticationType::NoThreeDs
                || payment_data.payment_attempt.payment_method
                    == Some(enums::PaymentMethodType::PayLater)
        }
        "PaymentStart" => {
            !matches!(
                payment_data.payment_intent.status,
                enums::IntentStatus::Failed | enums::IntentStatus::Succeeded
            ) && payment_data
                .connector_response
                .authentication_data
                .is_none()
        }
        "PaymentStatus" => {
            matches!(
                payment_data.payment_intent.status,
                enums::IntentStatus::Failed
                    | enums::IntentStatus::Processing
                    | enums::IntentStatus::Succeeded
                    | enums::IntentStatus::RequiresCustomerAction
            ) && payment_data.force_sync.unwrap_or(false)
        }
        "PaymentCancel" => matches!(
            payment_data.payment_intent.status,
            enums::IntentStatus::RequiresCapture
        ),
        "PaymentCapture" => {
            matches!(
                payment_data.payment_intent.status,
                enums::IntentStatus::RequiresCapture
            )
        }
        _ => false,
    }
}

pub async fn list_payments(
    db: &dyn Db,
    merchant: storage::MerchantAccount,
    constraints: api::PaymentListConstraints,
) -> RouterResponse<api::PaymentListResponse> {
    validate_payment_list_request(&constraints)?;
    let merchant_id = &merchant.merchant_id;
    let payment_intent = filter_by_constraints(db, &constraints, merchant_id)
        .await
        .map_err(|err| err.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))?;

    let data: Vec<api::PaymentsResponse> = payment_intent.into_iter().map(From::from).collect();
    utils::when(
        data.is_empty(),
        Err(errors::ApiErrorResponse::PaymentNotFound),
    )?;
    Ok(services::BachResponse::Json(api::PaymentListResponse {
        size: data.len(),
        data,
    }))
}

pub async fn add_process_sync_task(
    db: &dyn Db,
    payment_attempt: &storage::PaymentAttempt,
    schedule_time: time::PrimitiveDateTime,
) -> Result<(), errors::ProcessTrackerError> {
    let tracking_data = api::PaymentsRetrieveRequest {
        force_sync: true,
        merchant_id: Some(payment_attempt.merchant_id.clone()),

        resource_id: api::PaymentIdType::PaymentTxnId(payment_attempt.txn_id.clone()),
        param: None,
        connector: None,
    };
    let runner = "PAYMENTS_SYNC_WORKFLOW";
    let task = "PAYMENTS_SYNC";
    let process_tracker_id = pt_utils::get_process_tracker_id(
        runner,
        task,
        &payment_attempt.txn_id,
        &payment_attempt.merchant_id,
    );
    let process_tracker_entry = storage::ProcessTracker::make_process_tracker_new(
        process_tracker_id,
        task,
        runner,
        tracking_data,
        schedule_time,
    )?;

    db.insert_process(process_tracker_entry).await?;
    Ok(())
}
