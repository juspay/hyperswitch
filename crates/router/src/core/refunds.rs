pub mod validator;

use common_utils::ext_traits::AsyncExt;
use error_stack::{report, IntoReport, ResultExt};
use router_env::{instrument, tracing};
use scheduler::{consumer::types::process_data, utils as process_tracker_utils};

use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, RouterResponse, RouterResult, StorageErrorExt},
        payments::{self, access_token},
        utils as core_utils,
    },
    db, logger,
    routes::{metrics, AppState},
    services,
    types::{
        self,
        api::{self, refunds},
        domain,
        storage::{self, enums, ProcessTrackerExt},
        transformers::{ForeignFrom, ForeignInto},
    },
    utils::{self, OptionExt},
    workflows::payment_sync,
};

// ********************************************** REFUND EXECUTE **********************************************

#[instrument(skip_all)]
pub async fn refund_create_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: refunds::RefundRequest,
) -> RouterResponse<refunds::RefundResponse> {
    let db = &*state.store;
    let (merchant_id, payment_intent, payment_attempt, amount);

    merchant_id = &merchant_account.merchant_id;

    payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            &req.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    utils::when(
        payment_intent.status != enums::IntentStatus::Succeeded,
        || {
            Err(report!(errors::ApiErrorResponse::PaymentNotSucceeded)
                .attach_printable("unable to refund for a unsuccessful payment intent"))
        },
    )?;

    // Amount is not passed in request refer from payment intent.
    amount = req.amount.unwrap_or(
        payment_intent
            .amount_captured
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable("amount captured is none in a successful payment")?,
    );

    //[#299]: Can we change the flow based on some workflow idea
    utils::when(amount <= 0, || {
        Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
            field_name: "amount".to_string(),
            expected_format: "positive integer".to_string()
        })
        .attach_printable("amount less than or equal to zero"))
    })?;

    payment_attempt = db
        .find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
            &req.payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::SuccessfulPaymentNotFound)?;

    let creds_identifier = req
        .merchant_connector_details
        .as_ref()
        .map(|mcd| mcd.creds_identifier.to_owned());
    req.merchant_connector_details
        .to_owned()
        .async_map(|mcd| async {
            payments::helpers::insert_merchant_connector_creds_to_config(
                db,
                merchant_id.as_str(),
                mcd,
            )
            .await
        })
        .await
        .transpose()?;

    validate_and_create_refund(
        &state,
        &merchant_account,
        &key_store,
        &payment_attempt,
        &payment_intent,
        amount,
        req,
        creds_identifier,
    )
    .await
    .map(services::ApplicationResponse::Json)
}

#[instrument(skip_all)]
pub async fn trigger_refund_to_gateway(
    state: &AppState,
    refund: &storage::Refund,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    creds_identifier: Option<String>,
) -> RouterResult<storage::Refund> {
    let routed_through = payment_attempt
        .connector
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("Failed to retrieve connector from payment attempt")?;

    let storage_scheme = merchant_account.storage_scheme;
    metrics::REFUND_COUNT.add(
        &metrics::CONTEXT,
        1,
        &[metrics::request::add_attributes(
            "connector",
            routed_through.clone(),
        )],
    );

    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &routed_through,
        api::GetToken::Connector,
        payment_attempt.merchant_connector_id.clone(),
    )?;

    let currency = payment_attempt.currency.ok_or_else(|| {
        report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
            "Transaction in invalid. Missing field \"currency\" in payment_attempt.",
        )
    })?;

    validator::validate_for_valid_refunds(payment_attempt, connector.connector_name)?;

    let mut router_data = core_utils::construct_refund_router_data(
        state,
        &routed_through,
        merchant_account,
        key_store,
        (payment_attempt.amount, currency),
        payment_intent,
        payment_attempt,
        refund,
        creds_identifier,
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
            None,
        )
        .await
        .to_refund_failed_response()?
    } else {
        router_data
    };

    let refund_update = match router_data_res.response {
        Err(err) => storage::RefundUpdate::ErrorUpdate {
            refund_status: Some(enums::RefundStatus::Failure),
            refund_error_message: err.reason.or(Some(err.message)),
            refund_error_code: Some(err.code),
            updated_by: storage_scheme.to_string(),
        },
        Ok(response) => {
            if response.refund_status == diesel_models::enums::RefundStatus::Success {
                metrics::SUCCESSFUL_REFUND.add(
                    &metrics::CONTEXT,
                    1,
                    &[metrics::request::add_attributes(
                        "connector",
                        connector.connector_name.to_string(),
                    )],
                )
            }
            storage::RefundUpdate::Update {
                connector_refund_id: response.connector_refund_id,
                refund_status: response.refund_status,
                sent_to_gateway: true,
                refund_error_message: None,
                refund_arn: "".to_string(),
                updated_by: storage_scheme.to_string(),
            }
        }
    };

    let response = state
        .store
        .update_refund(
            refund.to_owned(),
            refund_update,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while updating refund: refund_id: {}",
                refund.refund_id
            )
        })?;
    Ok(response)
}

// ********************************************** REFUND SYNC **********************************************

pub async fn refund_response_wrapper<'a, F, Fut, T, Req>(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    request: Req,
    f: F,
) -> RouterResponse<refunds::RefundResponse>
where
    F: Fn(AppState, domain::MerchantAccount, domain::MerchantKeyStore, Req) -> Fut,
    Fut: futures::Future<Output = RouterResult<T>>,
    T: ForeignInto<refunds::RefundResponse>,
{
    Ok(services::ApplicationResponse::Json(
        f(state, merchant_account, key_store, request)
            .await?
            .foreign_into(),
    ))
}

#[instrument(skip_all)]
pub async fn refund_retrieve_core(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    request: refunds::RefundsRetrieveRequest,
) -> RouterResult<storage::Refund> {
    let refund_id = request.refund_id;
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
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let payment_id = refund.payment_id.as_str();
    payment_intent = db
        .find_payment_intent_by_payment_id_merchant_id(
            payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    payment_attempt = db
        .find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
            &refund.connector_transaction_id,
            payment_id,
            merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    let creds_identifier = request
        .merchant_connector_details
        .as_ref()
        .map(|mcd| mcd.creds_identifier.to_owned());
    request
        .merchant_connector_details
        .to_owned()
        .async_map(|mcd| async {
            payments::helpers::insert_merchant_connector_creds_to_config(
                db,
                merchant_id.as_str(),
                mcd,
            )
            .await
        })
        .await
        .transpose()?;

    response = if should_call_refund(&refund, request.force_sync.unwrap_or(false)) {
        sync_refund_with_gateway(
            &state,
            &merchant_account,
            &key_store,
            &payment_attempt,
            &payment_intent,
            &refund,
            creds_identifier,
        )
        .await
    } else {
        Ok(refund)
    }?;

    Ok(response)
}

fn should_call_refund(refund: &diesel_models::refund::Refund, force_sync: bool) -> bool {
    // This implies, we cannot perform a refund sync & `the connector_refund_id`
    // doesn't exist
    let predicate1 = refund.connector_refund_id.is_some();

    // This allows refund sync at connector level if force_sync is enabled, or
    // checks if the refund has failed
    let predicate2 = force_sync
        || !matches!(
            refund.refund_status,
            diesel_models::enums::RefundStatus::Failure
                | diesel_models::enums::RefundStatus::Success
        );

    predicate1 && predicate2
}

#[instrument(skip_all)]
pub async fn sync_refund_with_gateway(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund: &storage::Refund,
    creds_identifier: Option<String>,
) -> RouterResult<storage::Refund> {
    let connector_id = refund.connector.to_string();
    let connector: api::ConnectorData = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_id,
        api::GetToken::Connector,
        payment_attempt.connector.clone(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector")?;

    let storage_scheme = merchant_account.storage_scheme;

    let currency = payment_attempt.currency.get_required_value("currency")?;

    let mut router_data = core_utils::construct_refund_router_data::<api::RSync>(
        state,
        &connector_id,
        merchant_account,
        key_store,
        (payment_attempt.amount, currency),
        payment_intent,
        payment_attempt,
        refund,
        creds_identifier,
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
            None,
        )
        .await
        .to_refund_failed_response()?
    } else {
        router_data
    };

    let refund_update = match router_data_res.response {
        Err(error_message) => storage::RefundUpdate::ErrorUpdate {
            refund_status: None,
            refund_error_message: error_message.reason.or(Some(error_message.message)),
            refund_error_code: Some(error_message.code),
            updated_by: storage_scheme.to_string(),
        },
        Ok(response) => storage::RefundUpdate::Update {
            connector_refund_id: response.connector_refund_id,
            refund_status: response.refund_status,
            sent_to_gateway: true,
            refund_error_message: None,
            refund_arn: "".to_string(),
            updated_by: storage_scheme.to_string(),
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
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)
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
    state: AppState,
    merchant_account: domain::MerchantAccount,
    refund_id: &str,
    req: refunds::RefundUpdateRequest,
) -> RouterResponse<refunds::RefundResponse> {
    let db = state.store.as_ref();
    let refund = db
        .find_refund_by_merchant_id_refund_id(
            &merchant_account.merchant_id,
            refund_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let response = db
        .update_refund(
            refund,
            storage::RefundUpdate::MetadataAndReasonUpdate {
                metadata: req.metadata,
                reason: req.reason,
                updated_by: merchant_account.storage_scheme.to_string(),
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
#[allow(clippy::too_many_arguments)]
pub async fn validate_and_create_refund(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    refund_amount: i64,
    req: refunds::RefundRequest,
    creds_identifier: Option<String>,
) -> RouterResult<refunds::RefundResponse> {
    let db = &*state.store;

    // Only for initial dev and testing
    let refund_type = req.refund_type.unwrap_or_default();

    // If Refund Id not passed in request Generate one.

    let refund_id = core_utils::get_or_generate_id("refund_id", &req.refund_id, "ref")?;

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

    let connecter_transaction_id = payment_attempt.clone().connector_transaction_id.ok_or_else(|| {
        report!(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Transaction in invalid. Missing field \"connector_transaction_id\" in payment_attempt.")
    })?;

    let all_refunds = db
        .find_refund_by_merchant_id_connector_transaction_id(
            &merchant_account.merchant_id,
            &connecter_transaction_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let currency = payment_attempt.currency.get_required_value("currency")?;

    //[#249]: Add Connector Based Validation here.
    validator::validate_payment_order_age(&payment_intent.created_at, state.conf.refund.max_age)
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

    let connector = payment_attempt
        .connector
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .into_report()
        .attach_printable("No connector populated in payment attempt")?;

    let refund_create_req = storage::RefundNew::default()
        .set_refund_id(refund_id.to_string())
        .set_internal_reference_id(utils::generate_id(consts::ID_LENGTH, "refid"))
        .set_external_reference_id(Some(refund_id.clone()))
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
        .set_profile_id(payment_intent.profile_id.clone())
        .to_owned();

    let refund = match db
        .insert_refund(refund_create_req, merchant_account.storage_scheme)
        .await
    {
        Ok(refund) => {
            schedule_refund_execution(
                state,
                refund.clone(),
                refund_type,
                merchant_account,
                key_store,
                payment_attempt,
                payment_intent,
                creds_identifier,
            )
            .await?
        }
        Err(err) => {
            if err.current_context().is_db_unique_violation() {
                db.find_refund_by_merchant_id_refund_id(
                    merchant_account.merchant_id.as_str(),
                    refund_id.as_str(),
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?
            } else {
                return Err(err)
                    .change_context(errors::ApiErrorResponse::RefundNotFound)
                    .attach_printable("Inserting Refund failed");
            }
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
    state: AppState,
    merchant_account: domain::MerchantAccount,
    req: api_models::refunds::RefundListRequest,
) -> RouterResponse<api_models::refunds::RefundListResponse> {
    let db = state.store;
    let limit = validator::validate_refund_list(req.limit)?;
    let offset = req.offset.unwrap_or_default();

    let refund_list = db
        .filter_refund_by_constraints(
            &merchant_account.merchant_id,
            &req,
            merchant_account.storage_scheme,
            limit,
            offset,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    let data: Vec<refunds::RefundResponse> = refund_list
        .into_iter()
        .map(ForeignInto::foreign_into)
        .collect();

    let total_count = db
        .get_total_count_of_refunds(
            &merchant_account.merchant_id,
            &req,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    Ok(services::ApplicationResponse::Json(
        api_models::refunds::RefundListResponse {
            count: data.len(),
            total_count,
            data,
        },
    ))
}

#[instrument(skip_all)]
#[cfg(feature = "olap")]
pub async fn refund_filter_list(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    req: api_models::refunds::TimeRange,
) -> RouterResponse<api_models::refunds::RefundListMetaData> {
    let db = state.store;
    let filter_list = db
        .filter_refund_by_meta_constraints(
            &merchant_account.merchant_id,
            &req,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;

    Ok(services::ApplicationResponse::Json(filter_list))
}

impl ForeignFrom<storage::Refund> for api::RefundResponse {
    fn foreign_from(refund: storage::Refund) -> Self {
        let refund = refund;
        Self {
            payment_id: refund.payment_id,
            refund_id: refund.refund_id,
            amount: refund.refund_amount,
            currency: refund.currency.to_string(),
            reason: refund.refund_reason,
            status: refund.refund_status.foreign_into(),
            profile_id: refund.profile_id,
            metadata: refund.metadata,
            error_message: refund.refund_error_message,
            error_code: refund.refund_error_code,
            created_at: Some(refund.created_at),
            updated_at: Some(refund.updated_at),
            connector: refund.connector,
        }
    }
}

// ********************************************** PROCESS TRACKER **********************************************

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn schedule_refund_execution(
    state: &AppState,
    refund: storage::Refund,
    refund_type: api_models::refunds::RefundType,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: &storage::PaymentIntent,
    creds_identifier: Option<String>,
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
                                key_store,
                                payment_attempt,
                                payment_intent,
                                creds_identifier,
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

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &refund_core.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(&refund_core.merchant_id, &key_store)
        .await?;

    let response = refund_retrieve_core(
        state.clone(),
        merchant_account,
        key_store,
        refunds::RefundsRetrieveRequest {
            refund_id: refund_core.refund_internal_reference_id,
            force_sync: Some(true),
            merchant_connector_details: None,
        },
    )
    .await?;
    let terminal_status = [
        enums::RefundStatus::Success,
        enums::RefundStatus::Failure,
        enums::RefundStatus::TransactionFailure,
    ];
    match response.refund_status {
        status if terminal_status.contains(&status) => {
            let id = refund_tracker.id.clone();
            refund_tracker
                .clone()
                .finish_with_status(state.store.as_scheduler(), format!("COMPLETED_BY_PT_{id}"))
                .await?
        }
        _ => {
            _ = payment_sync::retry_sync_task(
                &*state.store,
                response.connector,
                response.merchant_id,
                refund_tracker.to_owned(),
            )
            .await?;
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

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &refund_core.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(&refund_core.merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let refund = db
        .find_refund_by_internal_reference_id_merchant_id(
            &refund_core.refund_internal_reference_id,
            &refund_core.merchant_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::RefundNotFound)?;
    match (&refund.sent_to_gateway, &refund.refund_status) {
        (false, enums::RefundStatus::Pending) => {
            let merchant_account = db
                .find_merchant_account_by_merchant_id(&refund.merchant_id, &key_store)
                .await
                .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

            let payment_attempt = db
                .find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
                    &refund.connector_transaction_id,
                    &refund_core.payment_id,
                    &refund.merchant_id,
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

            let payment_intent = db
                .find_payment_intent_by_payment_id_merchant_id(
                    &payment_attempt.payment_id,
                    &refund.merchant_id,
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

            //trigger refund request to gateway
            let updated_refund = trigger_refund_to_gateway(
                state,
                &refund,
                &merchant_account,
                &key_store,
                &payment_attempt,
                &payment_intent,
                None,
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
                .finish_with_status(db.as_scheduler(), format!("COMPLETED_BY_PT_{id}"))
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
        id: format!("{}_{}_{}", runner, task, refund.internal_reference_id),
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
        .to_duplicate_response(errors::ApiErrorResponse::DuplicateRefundRequest)
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
        id: format!("{}_{}_{}", runner, task, refund.internal_reference_id),
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
        .to_duplicate_response(errors::ApiErrorResponse::DuplicateRefundRequest)
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
        Some(s_time) => pt.retry(db.as_scheduler(), s_time).await,
        None => {
            pt.finish_with_status(db.as_scheduler(), "RETRIES_EXCEEDED".to_string())
                .await
        }
    }
}
