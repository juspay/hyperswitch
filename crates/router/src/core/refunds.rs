pub mod validator;

use error_stack::{report, IntoReport, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, RouterResponse, RouterResult, StorageErrorExt},
        payments::{self, access_token},
        utils as core_utils,
    },
    db, logger,
    routes::AppState,
    scheduler::{process_data, utils as process_tracker_utils, workflows::payment_sync},
    services,
    types::{
        self,
        api::{self, refunds},
        storage::{self, enums, ProcessTrackerExt},
        transformers::{ForeignFrom, ForeignInto},
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
    amount = req.amount.unwrap_or(payment_attempt.amount); // [#298]: Need to that capture amount
                                                           //[#299]: Can we change the flow based on some workflow idea
    utils::when(amount <= 0, || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount".to_string(),
            expected_format: "positive integer".to_string()
        })
        .attach_printable("amount less than zero"))
    })?;

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
        || {
            Err(report!(errors::ApiErrorResponse::PaymentNotSucceeded)
                .attach_printable("unable to refund for a unsuccessful payment intent"))
        },
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
    .map(services::ApplicationResponse::Json)
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
    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_id,
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector")?;

    let currency = payment_attempt.currency.ok_or_else(|| {
        report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "currency"
        })
        .attach_printable("Transaction in invalid")
    })?;

    validator::validate_for_valid_refunds(payment_attempt)?;

    let mut router_data = core_utils::construct_refund_router_data(
        state,
        &connector_id,
        merchant_account,
        (payment_attempt.amount, currency),
        payment_intent,
        payment_attempt,
        refund,
    )
    .await?;

    let add_access_token_result =
        access_token::add_access_token(state, &connector, merchant_account, &router_data).await?;

    logger::debug!(refund_router_data=?router_data);

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    let router_data_res = if !(add_access_token_result.connector_supports_access_token
        && router_data.access_token.is_none())
    {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::Execute,
            types::RefundsData,
            types::RefundsResponseData,
        > = connector.connector.get_connector_integration();
        services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            payments::CallConnectorAction::Trigger,
        )
        .await
        .map_err(|error| error.to_refund_failed_response())?
    } else {
        router_data
    };

    let refund_update = match router_data_res.response {
        Err(err) => storage::RefundUpdate::ErrorUpdate {
            refund_status: Some(enums::RefundStatus::Failure),
            refund_error_message: Some(err.message),
            refund_error_code: Some(err.code),
        },
        Ok(response) => storage::RefundUpdate::Update {
            connector_refund_id: response.connector_refund_id,
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
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while updating refund: refund_id: {}",
                refund.refund_id
            )
        })?;
    Ok(response)
}

// ********************************************** REFUND SYNC **********************************************

pub async fn refund_response_wrapper<'a, F, Fut, T>(
    state: &'a AppState,
    merchant_account: storage::MerchantAccount,
    refund_id: String,
    f: F,
) -> RouterResponse<refunds::RefundResponse>
where
    F: Fn(&'a AppState, storage::MerchantAccount, String) -> Fut,
    Fut: futures::Future<Output = RouterResult<T>>,
    T: ForeignInto<refunds::RefundResponse>,
{
    Ok(services::ApplicationResponse::Json(
        f(state, merchant_account, refund_id).await?.foreign_into(),
    ))
}

#[instrument(skip_all)]
pub async fn refund_retrieve_core(
    state: &AppState,
    merchant_account: storage::MerchantAccount,
    refund_id: String,
) -> RouterResult<storage::Refund> {
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
        .find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
            &refund.connector_transaction_id,
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

    Ok(response)
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
    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_id,
        api::GetToken::Connector,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector")?;

    let currency = payment_attempt.currency.get_required_value("currency")?;

    let mut router_data = core_utils::construct_refund_router_data::<api::RSync>(
        state,
        &connector_id,
        merchant_account,
        (payment_attempt.amount, currency),
        payment_intent,
        payment_attempt,
        refund,
    )
    .await?;

    let add_access_token_result =
        access_token::add_access_token(state, &connector, merchant_account, &router_data).await?;

    logger::debug!(refund_retrieve_router_data=?router_data);

    access_token::update_router_data_with_access_token_result(
        &add_access_token_result,
        &mut router_data,
        &payments::CallConnectorAction::Trigger,
    );

    let router_data_res = if !(add_access_token_result.connector_supports_access_token
        && router_data.access_token.is_none())
    {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            api::RSync,
            types::RefundsData,
            types::RefundsResponseData,
        > = connector.connector.get_connector_integration();
        services::execute_connector_processing_step(
            state,
            connector_integration,
            &router_data,
            payments::CallConnectorAction::Trigger,
        )
        .await
        .map_err(|error| error.to_refund_failed_response())?
    } else {
        router_data
    };

    let refund_update = match router_data_res.response {
        Err(error_message) => storage::RefundUpdate::ErrorUpdate {
            refund_status: None,
            refund_error_message: Some(error_message.message),
            refund_error_code: Some(error_message.code),
        },
        Ok(response) => storage::RefundUpdate::Update {
            connector_refund_id: response.connector_refund_id,
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
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Unable to update refund with refund_id: {}",
                refund.refund_id
            )
        })?;
    Ok(response)
}

// ********************************************** REFUND UPDATE **********************************************

pub async fn refund_update_core(
    db: &dyn db::StorageInterface,
    merchant_account: storage::MerchantAccount,
    refund_id: &str,
    req: refunds::RefundUpdateRequest,
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
            storage::RefundUpdate::MetadataAndReasonUpdate {
                metadata: req.metadata,
                reason: req.reason,
            },
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| format!("Unable to update refund with refund_id: {refund_id}"))?;

    Ok(services::ApplicationResponse::Json(response.foreign_into()))
}

// ********************************************** VALIDATIONS **********************************************

#[instrument(skip_all)]
pub async fn validate_and_create_refund(
    state: &AppState,
    merchant_account: &storage::merchant_account::MerchantAccount,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund_amount: i64,
    req: refunds::RefundRequest,
) -> RouterResult<refunds::RefundResponse> {
    let db = &*state.store;
    let (refund_id, all_refunds, currency, refund_create_req, refund);

    // Only for initial dev and testing
    let refund_type = req.refund_type.clone().unwrap_or_default();

    // If Refund Id not passed in request Generate one.

    refund_id = core_utils::get_or_generate_id("refund_id", &req.refund_id, "ref")?;

    let predicate = req
        .merchant_id
        .as_ref()
        .map(|merchant_id| merchant_id != &merchant_account.merchant_id);

    utils::when(predicate.unwrap_or(false), || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "merchant_id".to_string(),
            expected_format: "merchant_id from merchant account".to_string()
        })
        .attach_printable("invalid merchant_id in request"))
    })?;

    let refund = match validator::validate_uniqueness_of_refund_id_against_merchant_id(
        db,
        &payment_intent.payment_id,
        &merchant_account.merchant_id,
        &refund_id,
        merchant_account.storage_scheme,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "Unique violation while checking refund_id: {} against merchant_id: {}",
            refund_id, merchant_account.merchant_id
        )
    })? {
        Some(refund) => refund,
        None => {
            let connecter_transaction_id = match &payment_attempt.connector_transaction_id {
                Some(id) => id,
                None => "",
            };

            all_refunds = db
                .find_refund_by_merchant_id_connector_transaction_id(
                    &merchant_account.merchant_id,
                    connecter_transaction_id,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::RefundNotFound)
                .attach_printable("Failed to fetch refund")?;
            currency = payment_attempt.currency.get_required_value("currency")?;

            //[#249]: Add Connector Based Validation here.
            validator::validate_payment_order_age(
                &payment_intent.created_at,
                state.conf.refund.max_age,
            )
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "created_at".to_string(),
                expected_format: format!(
                    "created_at not older than {} days",
                    state.conf.refund.max_age,
                ),
            })?;

            validator::validate_refund_amount(payment_attempt.amount, &all_refunds, refund_amount)
                .change_context(errors::ApiErrorResponse::RefundAmountExceedsPaymentAmount)?;

            validator::validate_maximum_refund_against_payment_attempt(
                &all_refunds,
                state.conf.refund.max_attempts,
            )
            .change_context(errors::ApiErrorResponse::MaximumRefundCount)?;

            let connector = payment_attempt.connector.clone().ok_or_else(|| {
                report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("connector not populated in payment attempt.")
            })?;

            refund_create_req = storage::RefundNew::default()
                .set_refund_id(refund_id.to_string())
                .set_internal_reference_id(utils::generate_id(consts::ID_LENGTH, "refid"))
                .set_external_reference_id(Some(refund_id))
                .set_payment_id(req.payment_id)
                .set_merchant_id(merchant_account.merchant_id.clone())
                .set_connector_transaction_id(connecter_transaction_id.to_string())
                .set_connector(connector)
                .set_refund_type(req.refund_type.unwrap_or_default().foreign_into())
                .set_total_amount(payment_attempt.amount)
                .set_refund_amount(refund_amount)
                .set_currency(currency)
                .set_created_at(Some(common_utils::date_time::now()))
                .set_modified_at(Some(common_utils::date_time::now()))
                .set_refund_status(enums::RefundStatus::Pending)
                .set_metadata(req.metadata)
                .set_description(req.reason.clone())
                .set_attempt_id(payment_attempt.attempt_id.clone())
                .set_refund_reason(req.reason)
                .to_owned();

            refund = db
                .insert_refund(refund_create_req, merchant_account.storage_scheme)
                .await
                .map_err(|error| {
                    error.to_duplicate_response(errors::ApiErrorResponse::DuplicateRefundRequest)
                })?;
            schedule_refund_execution(
                state,
                refund,
                refund_type,
                merchant_account,
                payment_attempt,
                payment_intent,
            )
            .await?
        }
    };

    Ok(refund.foreign_into())
}

// ********************************************** Refund list **********************************************

///   If payment-id is provided, lists all the refunds associated with that particular payment-id
///   If payment-id is not provided, lists the refunds associated with that particular merchant - to the limit specified,if no limits given, it is 10 by default

#[instrument(skip_all)]
#[cfg(feature = "olap")]
pub async fn refund_list(
    db: &dyn db::StorageInterface,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: api_models::refunds::RefundListRequest,
) -> RouterResponse<api_models::refunds::RefundListResponse> {
    let limit = validator::validate_refund_list(req.limit)?;
    let refund_list = db
        .filter_refund_by_constraints(
            &merchant_account.merchant_id,
            &req,
            merchant_account.storage_scheme,
            limit,
        )
        .await
        .change_context(errors::ApiErrorResponse::RefundNotFound)?;

    let data: Vec<refunds::RefundResponse> = refund_list
        .into_iter()
        .map(ForeignInto::foreign_into)
        .collect();
    utils::when(data.is_empty(), || {
        Err(errors::ApiErrorResponse::RefundNotFound)
    })?;
    Ok(services::ApplicationResponse::Json(
        api_models::refunds::RefundListResponse { data },
    ))
}

impl ForeignFrom<storage::Refund> for api::RefundResponse {
    fn foreign_from(refund: storage::Refund) -> Self {
        let refund = refund;
        Self {
            payment_id: refund.payment_id,
            refund_id: refund.refund_id,
            amount: refund.refund_amount,
            currency: refund.currency.to_string(),
            reason: refund.description,
            status: refund.refund_status.foreign_into(),
            metadata: refund.metadata,
            error_message: refund.refund_error_message,
            error_code: refund.refund_error_code,
            created_at: Some(refund.created_at),
            updated_at: Some(refund.updated_at),
        }
    }
}

// ********************************************** PROCESS TRACKER **********************************************

#[instrument(skip_all)]
pub async fn schedule_refund_execution(
    state: &AppState,
    refund: storage::Refund,
    refund_type: api_models::refunds::RefundType,
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
                    // Execute the refund task based on refund_type
                    match refund_type {
                        api_models::refunds::RefundType::Scheduled => {
                            add_refund_execute_task(db, &refund, runner)
                                .await
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable_lazy(|| format!("Failed while pushing refund execute task to scheduler, refund_id: {}", refund.refund_id))?;

                            Ok(refund)
                        }
                        api_models::refunds::RefundType::Instant => {
                            trigger_refund_to_gateway(
                                state,
                                &refund,
                                merchant_account,
                                payment_attempt,
                                payment_intent,
                            )
                            .await
                        }
                    }
                }
                _ => {
                    // Sync the refund for status check
                    //[#300]: return refund status response
                    match refund_type {
                        api_models::refunds::RefundType::Scheduled => {
                            add_refund_sync_task(db, &refund, runner)
                                .await
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable_lazy(|| format!("Failed while pushing refund sync task in scheduler: refund_id: {}", refund.refund_id))?;
                            Ok(refund)
                        }
                        api_models::refunds::RefundType::Instant => {
                            // [#255]: This is not possible in schedule_refund_execution as it will always be scheduled
                            // sync_refund_with_gateway(data, &refund).await
                            Ok(refund)
                        }
                    }
                }
            }
        }
        //  [#255]: This is not allowed to be otherwise or all
        _ => Ok(refund),
    }?;
    Ok(result)
}

#[instrument(skip_all)]
pub async fn sync_refund_with_gateway_workflow(
    state: &AppState,
    refund_tracker: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
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

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(&refund_core.merchant_id)
        .await?;

    let response = refund_retrieve_core(
        state,
        merchant_account,
        refund_core.refund_internal_reference_id,
    )
    .await?;
    let terminal_status = vec![
        enums::RefundStatus::Success,
        enums::RefundStatus::Failure,
        enums::RefundStatus::TransactionFailure,
    ];
    match response.refund_status {
        status if terminal_status.contains(&status) => {
            let id = refund_tracker.id.clone();
            refund_tracker
                .clone()
                .finish_with_status(&*state.store, format!("COMPLETED_BY_PT_{id}"))
                .await?
        }
        _ => {
            payment_sync::retry_sync_task(
                &*state.store,
                response.connector,
                response.merchant_id,
                refund_tracker.to_owned(),
            )
            .await?
        }
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn start_refund_workflow(
    state: &AppState,
    refund_tracker: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    match refund_tracker.name.as_deref() {
        Some("EXECUTE_REFUND") => trigger_refund_execute_workflow(state, refund_tracker).await,
        Some("SYNC_REFUND") => sync_refund_with_gateway_workflow(state, refund_tracker).await,
        _ => Err(errors::ProcessTrackerError::JobNotFound),
    }
}

#[instrument(skip_all)]
pub async fn trigger_refund_execute_workflow(
    state: &AppState,
    refund_tracker: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
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
        (false, enums::RefundStatus::Pending) => {
            let merchant_account = db
                .find_merchant_account_by_merchant_id(&refund.merchant_id)
                .await
                .map_err(|error| {
                    error.to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
                })?;

            let payment_attempt = db
                .find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
                    &refund.connector_transaction_id,
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
            // create sync task
            add_refund_sync_task(db, &refund, "REFUND_WORKFLOW_ROUTER").await?;
        }
        (_, _) => {
            //mark task as finished
            let id = refund_tracker.id.clone();
            refund_tracker
                .clone()
                .finish_with_status(db, format!("COMPLETED_BY_PT_{id}"))
                .await?;
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
        connector_transaction_id: refund.connector_transaction_id.clone(),
        merchant_id: refund.merchant_id.clone(),
        payment_id: refund.payment_id.clone(),
    }
}

#[instrument(skip_all)]
pub async fn add_refund_sync_task(
    db: &dyn db::StorageInterface,
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
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while inserting task in process_tracker: refund_id: {}",
                refund.refund_id
            )
        })?;
    Ok(response)
}

#[instrument(skip_all)]
pub async fn add_refund_execute_task(
    db: &dyn db::StorageInterface,
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
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while inserting task in process_tracker: refund_id: {}",
                refund.refund_id
            )
        })?;
    Ok(response)
}

pub async fn get_refund_sync_process_schedule_time(
    db: &dyn db::StorageInterface,
    connector: &str,
    merchant_id: &str,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let redis_mapping: errors::CustomResult<process_data::ConnectorPTMapping, errors::RedisError> =
        db::get_and_deserialize_key(
            db,
            &format!("pt_mapping_refund_sync_{connector}"),
            "ConnectorPTMapping",
        )
        .await;

    let mapping = match redis_mapping {
        Ok(x) => x,
        Err(err) => {
            logger::error!("Error: while getting connector mapping: {}", err);
            process_data::ConnectorPTMapping::default()
        }
    };

    let time_delta =
        process_tracker_utils::get_schedule_time(mapping, merchant_id, retry_count + 1);

    Ok(process_tracker_utils::get_time_from_delta(time_delta))
}

pub async fn retry_refund_sync_task(
    db: &dyn db::StorageInterface,
    connector: String,
    merchant_id: String,
    pt: storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let schedule_time =
        get_refund_sync_process_schedule_time(db, &connector, &merchant_id, pt.retry_count).await?;

    match schedule_time {
        Some(s_time) => pt.retry(db, s_time).await,
        None => {
            pt.finish_with_status(db, "RETRIES_EXCEEDED".to_string())
                .await
        }
    }
}
