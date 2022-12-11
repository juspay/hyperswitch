pub mod validator;

use error_stack::{report, IntoReport, ResultExt};
use router_env::tracing::{self, instrument};
use uuid::Uuid;

use crate::{
    core::{
        errors::{self, ConnectorErrorExt, RouterResponse, RouterResult, StorageErrorExt},
        payments, utils as core_utils,
    },
    db::StorageInterface,
    logger,
    routes::AppState,
    services,
    types::{
        self,
        api::{self, refunds},
        storage::{self, enums},
    },
    utils::{self, OptionExt},
};

// ********************************************** REFUND EXECUTE **********************************************

#[instrument(skip_all)]
pub async fn refund_create_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: refunds::RefundRequest,
) -> RouterResponse<refunds::RefundResponse> {
    let db = &*state.store;
    let (merchant_id, payment_intent, payment_attempt, amount);

    merchant_id = &merchant_account.merchant_id;

    payment_attempt = db
        .find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
            &req.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::SuccessfulPaymentNotFound)?;

    // Amount is not passed in request refer from payment attempt.
    amount = req.amount.unwrap_or(payment_attempt.amount); // FIXME: Need to that capture amount

    //TODO: Can we change the flow based on some workflow idea
    utils::when(
        amount <= 0,
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount".to_string(),
            expected_format: "positive integer".to_string()
        })
        .attach_printable("amount less than zero")),
    )?;

    payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &req.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

    utils::when(
        payment_intent.status != enums::IntentStatus::Succeeded,
        Err(report!(errors::ApiErrorResponse::PaymentNotSucceeded)
            .attach_printable("unable to refund for a unsuccessful payment intent")),
    )?;

    validate_and_create_refund(
        state,
        &merchant_account,
        &payment_attempt,
        &payment_intent,
        amount,
        req,
    )
    .await
    .map(services::BachResponse::Json)
}

#[instrument(skip_all)]
pub async fn trigger_refund_to_gateway(
    state: &AppState,
    refund: &storage::Refund,
    merchant_account: &storage::merchant_account::MerchantAccount,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
) -> RouterResult<storage::Refund> {
    let connector = payment_attempt
        .connector
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)?;
    let connector_id = connector.to_string();
    let connector: api::ConnectorData =
        api::ConnectorData::get_connector_by_name(&state.conf.connectors, &connector_id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get the connector")?;

    let currency = payment_attempt.currency.ok_or_else(|| {
        report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "currency".to_string()
        })
        .attach_printable("Transaction in invalid")
    })?;

    let router_data = core_utils::construct_refund_router_data(
        state,
        &connector_id,
        merchant_account,
        (payment_attempt.amount, currency),
        None,
        payment_intent,
        payment_attempt,
        refund,
    )
    .await?;

    logger::debug!(?router_data);
    let connector_integration: services::BoxedConnectorIntegration<
        api::Execute,
        types::RefundsData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();
    let router_data = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .map_err(|error| error.to_refund_failed_response())?;

    let refund_update = match router_data.response {
        Err(err) => storage::RefundUpdate::ErrorUpdate {
            refund_status: Some(enums::RefundStatus::Failure),
            refund_error_message: Some(err.message),
        },
        Ok(response) => storage::RefundUpdate::Update {
            pg_refund_id: response.connector_refund_id,
            refund_status: response.refund_status,
            sent_to_gateway: true,
            refund_error_message: None,
            refund_arn: "".to_string(),
        },
    };

    let response = state
        .store
        .update_refund(
            refund.to_owned(),
            refund_update,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(response)
}

// ********************************************** REFUND SYNC **********************************************

#[instrument(skip_all)]
pub async fn refund_retrieve_core(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    refund_id: String,
) -> RouterResponse<refunds::RefundResponse> {
    let db = &*state.store;
    let (merchant_id, payment_intent, payment_attempt, refund, response);

    merchant_id = &merchant_account.merchant_id;

    refund = db
        .find_refund_by_merchant_id_refund_id(
            merchant_id,
            refund_id.as_str(),
            merchant_account.storage_scheme,
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::RefundNotFound))?;

    let payment_id = refund.payment_id.as_str();
    payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))?;

    payment_attempt = db
        .find_payment_attempt_by_transaction_id_payment_id_merchant_id(
            &refund.transaction_id,
            payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound))?;

    response = sync_refund_with_gateway(
        state,
        &merchant_account,
        &payment_attempt,
        &payment_intent,
        &refund,
    )
    .await?;

    Ok(services::BachResponse::Json(response.into()))
}

#[instrument(skip_all)]
pub async fn sync_refund_with_gateway(
    state: &AppState,
    merchant_account: &storage::MerchantAccount,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund: &storage::Refund,
) -> RouterResult<storage::Refund> {
    let connector_id = refund.connector.to_string();
    let connector: api::ConnectorData =
        api::ConnectorData::get_connector_by_name(&state.conf.connectors, &connector_id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get the connector")?;

    let currency = payment_attempt.currency.get_required_value("currency")?;

    let router_data = core_utils::construct_refund_router_data::<api::RSync>(
        state,
        &connector_id,
        merchant_account,
        (payment_attempt.amount, currency),
        None,
        payment_intent,
        payment_attempt,
        refund,
    )
    .await?;

    let connector_integration: services::BoxedConnectorIntegration<
        api::RSync,
        types::RefundsData,
        types::RefundsResponseData,
    > = connector.connector.get_connector_integration();
    let router_data = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
    )
    .await
    .map_err(|error| error.to_refund_failed_response())?;

    let refund_update = match router_data.response {
        Err(error_message) => storage::RefundUpdate::ErrorUpdate {
            refund_status: None,
            refund_error_message: Some(error_message.message),
        },
        Ok(response) => storage::RefundUpdate::Update {
            pg_refund_id: response.connector_refund_id,
            refund_status: response.refund_status,
            sent_to_gateway: true,
            refund_error_message: None,
            refund_arn: "".to_string(),
        },
    };

    let response = state
        .store
        .update_refund(
            refund.to_owned(),
            refund_update,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(response)
}

// ********************************************** REFUND UPDATE **********************************************

pub async fn refund_update_core(
    db: &dyn StorageInterface,
    merchant_account: storage::MerchantAccount,
    refund_id: &str,
    req: refunds::RefundRequest,
) -> RouterResponse<refunds::RefundResponse> {
    let refund = db
        .find_refund_by_merchant_id_refund_id(
            &merchant_account.merchant_id,
            refund_id,
            merchant_account.storage_scheme,
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::RefundNotFound))?;

    let response = db
        .update_refund(
            refund,
            storage::RefundUpdate::MetadataUpdate {
                metadata: req.metadata,
            },
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(services::BachResponse::Json(response.into()))
}

// ********************************************** VALIDATIONS **********************************************

#[instrument(skip_all)]
pub async fn validate_and_create_refund(
    state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund_amount: i32,
    req: refunds::RefundRequest,
) -> RouterResult<refunds::RefundResponse> {
    let db = &*state.store;
    let (refund_id, all_refunds, currency, refund_create_req, refund);

    // Only for initial dev and testing
    let force_process = req.force_process.unwrap_or(false);

    // If Refund Id not passed in request Generate one.

    refund_id = core_utils::get_or_generate_id("refund_id", &req.refund_id, "ref")?;

    let predicate = req
        .merchant_id
        .as_ref()
        .map(|merchant_id| merchant_id != &merchant_account.merchant_id);

    utils::when(
        predicate.unwrap_or(false),
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "merchant_id".to_string(),
            expected_format: "merchant_id from merchant account".to_string()
        })
        .attach_printable("invalid merchant_id in request")),
    )?;

    let refund = match validator::validate_uniqueness_of_refund_id_against_merchant_id(
        db,
        &payment_intent.payment_id,
        &merchant_account.merchant_id,
        &refund_id,
        merchant_account.storage_scheme,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?
    {
        Some(refund) => refund,
        None => {
            let connecter_transaction_id = match &payment_attempt.connector_transaction_id {
                Some(id) => id,
                None => "",
            };

            all_refunds = db
                .find_refund_by_merchant_id_transaction_id(
                    &merchant_account.merchant_id,
                    connecter_transaction_id,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::RefundNotFound)
                .attach_printable("Failed to fetch refund")?;
            currency = payment_attempt.currency.get_required_value("currency")?;

            // TODO: Add Connector Based Validation here.
            validator::validate_payment_order_age(&payment_intent.created_at).change_context(
                errors::ApiErrorResponse::InvalidDataFormat {
                    field_name: "created_at".to_string(),
                    expected_format: format!(
                        "created_at not older than {} days",
                        validator::REFUND_MAX_AGE
                    ),
                },
            )?;

            validator::validate_refund_amount(payment_attempt.amount, &all_refunds, refund_amount)
                .change_context(errors::ApiErrorResponse::RefundAmountExceedsPaymentAmount)?;

            validator::validate_maximum_refund_against_payment_attempt(&all_refunds)
                .change_context(errors::ApiErrorResponse::MaximumRefundCount)?;

            let connector = payment_attempt
                .connector
                .clone()
                .ok_or(errors::ApiErrorResponse::InternalServerError)?;

            refund_create_req = mk_new_refund(
                req,
                connector,
                payment_attempt,
                currency,
                &refund_id,
                &merchant_account.merchant_id,
                refund_amount,
            );

            refund = db
                .insert_refund(refund_create_req, merchant_account.storage_scheme)
                .await
                .map_err(|error| {
                    error.to_duplicate_response(errors::ApiErrorResponse::DuplicateRefundRequest)
                })?;
            schedule_refund_execution(
                state,
                refund,
                force_process, // *force_process,
                merchant_account,
                payment_attempt,
                payment_intent,
            )
            .await?
        }
    };
    Ok(refund.into())
}

// ********************************************** UTILS **********************************************

// FIXME: function should not have more than 3 arguments.
// Consider to use builder pattern.
#[instrument]
fn mk_new_refund(
    request: refunds::RefundRequest,
    connector: String,
    payment_attempt: &storage::PaymentAttempt,
    currency: enums::Currency,
    refund_id: &str,
    merchant_id: &str,
    refund_amount: i32,
) -> storage::RefundNew {
    let current_time = common_utils::date_time::now();
    let connecter_transaction_id = match &payment_attempt.connector_transaction_id {
        Some(id) => id,
        None => "",
    };
    storage::RefundNew {
        refund_id: refund_id.to_string(),
        internal_reference_id: Uuid::new_v4().to_string(),
        external_reference_id: Some(refund_id.to_string()),
        payment_id: request.payment_id,
        merchant_id: merchant_id.to_string(),
        // FIXME: remove the default.
        transaction_id: connecter_transaction_id.to_string(),
        connector,
        refund_type: enums::RefundType::RegularRefund,
        total_amount: refund_amount,
        currency,
        refund_amount,
        created_at: Some(current_time),
        modified_at: Some(current_time),
        refund_status: enums::RefundStatus::Pending,
        metadata: request.metadata,
        description: request.reason,
        ..storage::RefundNew::default()
    }
}

impl<F> TryFrom<types::RefundsRouterData<F>> for refunds::RefundResponse {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(data: types::RefundsRouterData<F>) -> RouterResult<Self> {
        let refund_id = data.request.refund_id.to_string();
        let response = data.response;

        let (status, error_message) = match response {
            Ok(response) => (response.refund_status.into(), None),
            Err(error_response) => (api::RefundStatus::Pending, Some(error_response.message)),
        };

        Ok(refunds::RefundResponse {
            payment_id: data.payment_id,
            refund_id,
            amount: data.request.amount / 100,
            currency: data.request.currency.to_string(),
            reason: Some("TODO: Not propagated".to_string()), // TODO: Not propagated
            status,
            metadata: None,
            error_message,
        })
    }
}

impl From<storage::Refund> for api::RefundResponse {
    fn from(refund: storage::Refund) -> Self {
        Self {
            payment_id: refund.payment_id,
            refund_id: refund.refund_id,
            amount: refund.refund_amount,
            currency: refund.currency.to_string(),
            reason: refund.description,
            status: refund.refund_status.into(),
            metadata: refund.metadata,
            error_message: refund.refund_error_message,
        }
    }
}

// ********************************************** PROCESS TRACKER **********************************************

#[instrument(skip_all)]
pub async fn schedule_refund_execution(
    state: &AppState,
    refund: storage::Refund,
    //FIXME: change to refund_Type here
    force_process: bool,
    merchant_account: &storage::merchant_account::MerchantAccount,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
) -> RouterResult<storage::Refund> {
    // refunds::RefundResponse> {
    let db = &*state.store;
    let runner = "REFUND_WORKFLOW_ROUTER";
    let task = "EXECUTE_REFUND";
    let task_id = format!("{}_{}_{}", runner, task, refund.internal_reference_id);

    let refund_process = db
        .find_process_by_id(&task_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to find the process id")?;
    let result = match refund.refund_status {
        enums::RefundStatus::Pending | enums::RefundStatus::ManualReview => {
            match (refund.sent_to_gateway, refund_process) {
                (false, None) => {
                    //execute
                    if force_process {
                        trigger_refund_to_gateway(
                            state,
                            &refund,
                            merchant_account,
                            payment_attempt,
                            payment_intent,
                        )
                        .await
                    } else {
                        add_refund_execute_task(db, &refund, runner)
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)?;
                        Ok(refund)
                    }
                }
                _ => {
                    //sync status
                    //TODO: return refund status response
                    if force_process {
                        // sync_refund_with_gateway(data, &refund).await
                        Ok(refund)
                    } else {
                        add_refund_sync_task(db, &refund, runner)
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)?;
                        Ok(refund)
                    }
                }
            }
        }
        //  FIXME: THis is not allowed to be otherwise or all
        _ => Ok(refund),
    }?;
    Ok(result)
}

#[instrument(skip_all)]
pub async fn sync_refund_with_gateway_workflow(
    db: &dyn StorageInterface,
    refund_tracker: &storage::ProcessTracker,
) -> RouterResult<()> {
    let refund_core =
        serde_json::from_value::<storage::RefundCoreWorkflow>(refund_tracker.tracking_data.clone())
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!(
                    "unable to convert into refund_core {:?}",
                    refund_tracker.tracking_data
                )
            })?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&refund_core.merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    // FIXME we actually don't use this?
    let _refund = db
        .find_refund_by_internal_reference_id_merchant_id(
            &refund_core.refund_internal_reference_id,
            &refund_core.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::RefundNotFound))?;

    Ok(())
    // sync_refund_with_gateway(data, &refund).await.map(|_| ())
}

#[instrument(skip_all)]
pub async fn start_refund_workflow(
    state: &AppState,
    refund_tracker: &storage::ProcessTracker,
) -> RouterResult<()> {
    match refund_tracker.name.as_deref() {
        Some("EXECUTE_REFUND") => trigger_refund_execute_workflow(state, refund_tracker).await,
        Some("SYNC_REFUND") => {
            sync_refund_with_gateway_workflow(&*state.store, refund_tracker).await
        }
        _ => Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Job name cannot be identified")),
    }
}

#[instrument(skip_all)]
pub async fn trigger_refund_execute_workflow(
    state: &AppState,
    refund_tracker: &storage::ProcessTracker,
) -> RouterResult<()> {
    let db = &*state.store;
    let refund_core =
        serde_json::from_value::<storage::RefundCoreWorkflow>(refund_tracker.tracking_data.clone())
            .into_report()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!(
                    "unable to convert into refund_core {:?}",
                    refund_tracker.tracking_data
                )
            })?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&refund_core.merchant_id)
        .await
        .map_err(|error| {
            error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        })?;

    let refund = db
        .find_refund_by_internal_reference_id_merchant_id(
            &refund_core.refund_internal_reference_id,
            &refund_core.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .map_err(|error| error.to_not_found_response(errors::ApiErrorResponse::RefundNotFound))?;
    match (&refund.sent_to_gateway, &refund.refund_status) {
        //FIXME: Conversion should come from trait
        (false, enums::RefundStatus::Pending) => {
            let merchant_account = db
                .find_merchant_account_by_merchant_id(&refund.merchant_id)
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
                })?;

            let payment_attempt = db
                .find_payment_attempt_by_transaction_id_payment_id_merchant_id(
                    &refund.transaction_id,
                    &refund_core.payment_id,
                    &refund.merchant_id,
                    merchant_account.storage_scheme,
                )
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
                })?;

            let payment_intent = db
                .find_payment_intent_by_payment_id_merchant_id(
                    &payment_attempt.payment_id,
                    &refund.merchant_id,
                    merchant_account.storage_scheme,
                )
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
                })?;

            //trigger refund request to gateway
            let updated_refund = trigger_refund_to_gateway(
                state,
                &refund,
                &merchant_account,
                &payment_attempt,
                &payment_intent,
            )
            .await?;
            add_refund_sync_task(db, &updated_refund, "REFUND_WORKFLOW_ROUTER").await?;
        }
        (true, enums::RefundStatus::Pending) => {
            //create sync task
            add_refund_sync_task(db, &refund, "REFUND_WORKFLOW_ROUTER").await?;
        }
        (_, _) => {
            //mark task as finished
        }
    };
    Ok(())
}

#[instrument]
pub fn refund_to_refund_core_workflow_model(
    refund: &storage::Refund,
) -> storage::RefundCoreWorkflow {
    storage::RefundCoreWorkflow {
        refund_internal_reference_id: refund.internal_reference_id.clone(),
        transaction_id: refund.transaction_id.clone(),
        merchant_id: refund.merchant_id.clone(),
        payment_id: refund.payment_id.clone(),
    }
}

#[instrument(skip_all)]
pub async fn add_refund_sync_task(
    db: &dyn StorageInterface,
    refund: &storage::Refund,
    runner: &str,
) -> RouterResult<storage::ProcessTracker> {
    let current_time = common_utils::date_time::now();
    let refund_workflow_model = serde_json::to_value(refund_to_refund_core_workflow_model(refund))
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("unable to convert into value {:?}", &refund))?;
    let task = "SYNC_REFUND";
    let process_tracker_entry = storage::ProcessTrackerNew {
        id: format!("{}_{}_{}", runner, task, refund.id),
        name: Some(String::from(task)),
        tag: vec![String::from("REFUND")],
        runner: Some(String::from(runner)),
        retry_count: 0,
        schedule_time: Some(common_utils::date_time::now()),
        rule: String::new(),
        tracking_data: refund_workflow_model,
        business_status: String::from("Pending"),
        status: enums::ProcessTrackerStatus::New,
        event: vec![],
        created_at: current_time,
        updated_at: current_time,
    };

    let response = db
        .insert_process(process_tracker_entry)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(response)
}

#[instrument(skip_all)]
pub async fn add_refund_execute_task(
    db: &dyn StorageInterface,
    refund: &storage::Refund,
    runner: &str,
) -> RouterResult<storage::ProcessTracker> {
    let task = "EXECUTE_REFUND";
    let current_time = common_utils::date_time::now();
    let refund_workflow_model = serde_json::to_value(refund_to_refund_core_workflow_model(refund))
        .into_report()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("unable to convert into value {:?}", &refund))?;
    let process_tracker_entry = storage::ProcessTrackerNew {
        id: format!("{}_{}_{}", runner, task, refund.id),
        name: Some(String::from(task)),
        tag: vec![String::from("REFUND")],
        runner: Some(String::from(runner)),
        retry_count: 0,
        schedule_time: Some(common_utils::date_time::now()),
        rule: String::new(),
        tracking_data: refund_workflow_model,
        business_status: String::from("Pending"),
        status: enums::ProcessTrackerStatus::New,
        event: vec![],
        created_at: current_time,
        updated_at: current_time,
    };

    let response = db
        .insert_process(process_tracker_entry)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    Ok(response)
}
