use common_utils::ext_traits::{OptionExt, StringExt, ValueExt};
use error_stack::ResultExt;
use router_env::logger;
use scheduler::{
    consumer::{self, types::process_data, workflows::ProcessTrackerWorkflow},
    errors as sch_errors, utils as scheduler_utils, SchedulerAppState,
};

use crate::{
    consts,
    core::{
        errors::StorageErrorExt,
        payments::{self as payment_flows, operations},
    },
    db::StorageInterface,
    errors,
    routes::AppState,
    services,
    types::{
        api,
        storage::{self, enums},
    },
    utils,
};

pub struct PaymentsSyncWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for PaymentsSyncWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), sch_errors::ProcessTrackerError> {
        let db: &dyn StorageInterface = &*state.store;
        let tracking_data: api::PaymentsRetrieveRequest = process
            .tracking_data
            .clone()
            .parse_value("PaymentsRetrieveRequest")?;

        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                tracking_data
                    .merchant_id
                    .as_ref()
                    .get_required_value("merchant_id")?,
                &db.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(
                tracking_data
                    .merchant_id
                    .as_ref()
                    .get_required_value("merchant_id")?,
                &key_store,
            )
            .await?;

        // TODO: Add support for ReqState in PT flows
        let (mut payment_data, _, customer, _, _) = Box::pin(
            payment_flows::payments_operation_core::<api::PSync, _, _, _>(
                state,
                state.get_req_state(),
                merchant_account.clone(),
                key_store.clone(),
                operations::PaymentStatus,
                tracking_data.clone(),
                payment_flows::CallConnectorAction::Trigger,
                services::AuthFlow::Client,
                None,
                api::HeaderPayload::default(),
            ),
        )
        .await?;

        let terminal_status = [
            enums::AttemptStatus::RouterDeclined,
            enums::AttemptStatus::Charged,
            enums::AttemptStatus::AutoRefunded,
            enums::AttemptStatus::Voided,
            enums::AttemptStatus::VoidFailed,
            enums::AttemptStatus::CaptureFailed,
            enums::AttemptStatus::Failure,
        ];
        match &payment_data.payment_attempt.status {
            status if terminal_status.contains(status) => {
                state
                    .get_db()
                    .as_scheduler()
                    .finish_process_with_business_status(process, "COMPLETED_BY_PT".to_string())
                    .await?
            }
            _ => {
                let connector = payment_data
                    .payment_attempt
                    .connector
                    .clone()
                    .ok_or(sch_errors::ProcessTrackerError::MissingRequiredField)?;

                let is_last_retry = retry_sync_task(
                    db,
                    connector,
                    payment_data.payment_attempt.merchant_id.clone(),
                    process,
                )
                .await?;

                // If the payment status is still processing and there is no connector transaction_id
                // then change the payment status to failed if all retries exceeded
                if is_last_retry
                    && payment_data.payment_attempt.status == enums::AttemptStatus::Pending
                    && payment_data
                        .payment_attempt
                        .connector_transaction_id
                        .as_ref()
                        .is_none()
                {
                    let payment_intent_update = hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::PGStatusUpdate { status: api_models::enums::IntentStatus::Failed,updated_by: merchant_account.storage_scheme.to_string(), incremental_authorization_allowed: Some(false) };
                    let payment_attempt_update =
                        hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::ErrorUpdate {
                            connector: None,
                            status: api_models::enums::AttemptStatus::Failure,
                            error_code: None,
                            error_message: None,
                            error_reason: Some(Some(
                                consts::REQUEST_TIMEOUT_ERROR_MESSAGE_FROM_PSYNC.to_string(),
                            )),
                            amount_capturable: Some(common_utils::types::MinorUnit::new(0)),
                            updated_by: merchant_account.storage_scheme.to_string(),
                            unified_code: None,
                            unified_message: None,
                            connector_transaction_id: None,
                            payment_method_data: None,
                        };

                    payment_data.payment_attempt = db
                        .update_payment_attempt_with_attempt_id(
                            payment_data.payment_attempt,
                            payment_attempt_update,
                            merchant_account.storage_scheme,
                        )
                        .await
                        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

                    payment_data.payment_intent = db
                        .update_payment_intent(
                            payment_data.payment_intent,
                            payment_intent_update,
                            merchant_account.storage_scheme,
                        )
                        .await
                        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

                    let profile_id = payment_data
                        .payment_intent
                        .profile_id
                        .as_ref()
                        .get_required_value("profile_id")
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Could not find profile_id in payment intent")?;

                    let business_profile = db
                        .find_business_profile_by_profile_id(profile_id)
                        .await
                        .to_not_found_response(
                            errors::ApiErrorResponse::BusinessProfileNotFound {
                                id: profile_id.to_string(),
                            },
                        )?;

                    // Trigger the outgoing webhook to notify the merchant about failed payment
                    let operation = operations::PaymentStatus;
                    Box::pin(utils::trigger_payments_webhook(
                        merchant_account,
                        business_profile,
                        &key_store,
                        payment_data,
                        customer,
                        state,
                        operation,
                    ))
                    .await
                    .map_err(|error| logger::warn!(payments_outgoing_webhook_error=?error))
                    .ok();
                }
            }
        };
        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
        error: sch_errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), sch_errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}

/// Get the next schedule time
///
/// The schedule time can be configured in configs by this key `pt_mapping_trustpay`
/// ```json
/// {
///     "default_mapping": {
///         "start_after": 60,
///         "frequency": [300],
///         "count": [5]
///     },
///     "max_retries_count": 5
/// }
/// ```
///
/// This config represents
///
/// `start_after`: The first psync should happen after 60 seconds
///
/// `frequency` and `count`: The next 5 retries should have an interval of 300 seconds between them
///
pub async fn get_sync_process_schedule_time(
    db: &dyn StorageInterface,
    connector: &str,
    merchant_id: &str,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let mapping: common_utils::errors::CustomResult<
        process_data::ConnectorPTMapping,
        errors::StorageError,
    > = db
        .find_config_by_key(&format!("pt_mapping_{connector}"))
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("ConnectorPTMapping")
                .change_context(errors::StorageError::DeserializationFailed)
        });
    let mapping = match mapping {
        Ok(x) => x,
        Err(error) => {
            logger::info!(?error, "Redis Mapping Error");
            process_data::ConnectorPTMapping::default()
        }
    };
    let time_delta = scheduler_utils::get_schedule_time(mapping, merchant_id, retry_count);

    Ok(scheduler_utils::get_time_from_delta(time_delta))
}

/// Schedule the task for retry
///
/// Returns bool which indicates whether this was the last retry or not
pub async fn retry_sync_task(
    db: &dyn StorageInterface,
    connector: String,
    merchant_id: String,
    pt: storage::ProcessTracker,
) -> Result<bool, sch_errors::ProcessTrackerError> {
    let schedule_time =
        get_sync_process_schedule_time(db, &connector, &merchant_id, pt.retry_count + 1).await?;

    match schedule_time {
        Some(s_time) => {
            db.as_scheduler().retry_process(pt, s_time).await?;
            Ok(false)
        }
        None => {
            db.as_scheduler()
                .finish_process_with_business_status(pt, "RETRIES_EXCEEDED".to_string())
                .await?;
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_get_default_schedule_time() {
        let schedule_time_delta =
            scheduler_utils::get_schedule_time(process_data::ConnectorPTMapping::default(), "-", 0)
                .unwrap();
        let first_retry_time_delta =
            scheduler_utils::get_schedule_time(process_data::ConnectorPTMapping::default(), "-", 1)
                .unwrap();
        let cpt_default = process_data::ConnectorPTMapping::default().default_mapping;
        assert_eq!(
            vec![schedule_time_delta, first_retry_time_delta],
            vec![
                cpt_default.start_after,
                cpt_default.frequencies.first().unwrap().0
            ]
        );
    }
}
