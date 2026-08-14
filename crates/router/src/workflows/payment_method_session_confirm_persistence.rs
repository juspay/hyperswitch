use common_utils::ext_traits::ValueExt;
use error_stack::{Report, ResultExt};
use scheduler::{
    consumer::{self, types::process_data},
    utils as pt_utils,
    workflows::ProcessTrackerWorkflow,
};

use crate::{
    core::{errors::RouterResult, payment_methods},
    errors, logger,
    routes::{metrics, SessionState},
    types::{domain, storage},
};

const COMPLETED_BY_PT_STATUS: &str = "COMPLETED_BY_PT";
const RETRIES_EXCEEDED_STATUS: &str = "RETRIES_EXCEEDED";

pub struct PaymentMethodSessionConfirmPersistenceWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentMethodSessionConfirmPersistenceWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data: storage::PaymentMethodSessionConfirmPersistenceTrackingData = process
            .tracking_data
            .clone()
            .parse_value("PaymentMethodSessionConfirmPersistenceTrackingData")?;

        let result = persist_payment_method(state, &tracking_data).await;
        finish_task(state, process, &tracking_data, result).await
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}

async fn persist_payment_method(
    state: &SessionState,
    tracking_data: &storage::PaymentMethodSessionConfirmPersistenceTrackingData,
) -> RouterResult<common_utils::id_type::GlobalPaymentMethodId> {
    let db = &*state.store;
    let provider_key_store = db
        .get_merchant_key_store_by_merchant_id(
            &tracking_data.provider_merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch merchant key store for PM persistence")?;
    let provider_merchant_account = db
        .find_merchant_account_by_merchant_id(
            &tracking_data.provider_merchant_id,
            &provider_key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch provider merchant account for PM persistence")?;

    let (processor_merchant_account, processor_key_store) = if tracking_data.processor_merchant_id
        == tracking_data.provider_merchant_id
    {
        (
            provider_merchant_account.clone(),
            provider_key_store.clone(),
        )
    } else {
        let processor_key_store = db
            .get_merchant_key_store_by_merchant_id(
                &tracking_data.processor_merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch processor key store for PM persistence")?;
        let processor_merchant_account = db
            .find_merchant_account_by_merchant_id(
                &tracking_data.processor_merchant_id,
                &processor_key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch processor merchant account for PM persistence")?;
        (processor_merchant_account, processor_key_store)
    };

    let profile = db
        .find_business_profile_by_merchant_id_profile_id(
            &processor_key_store,
            &tracking_data.processor_merchant_id,
            &tracking_data.profile_id,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch profile for PM persistence")?;
    let platform = domain::Platform::new(
        provider_merchant_account,
        provider_key_store,
        processor_merchant_account,
        processor_key_store,
        None,
    );

    payment_methods::persist_payment_method_session_confirm_fast_path(
        state,
        &platform,
        &profile,
        tracking_data,
    )
    .await
}

async fn finish_task(
    state: &SessionState,
    process: storage::ProcessTracker,
    tracking_data: &storage::PaymentMethodSessionConfirmPersistenceTrackingData,
    result: Result<common_utils::id_type::GlobalPaymentMethodId, Report<errors::ApiErrorResponse>>,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    match result {
        Ok(persisted_payment_method_id) => {
            metrics::PAYMENT_METHOD_SESSION_CONFIRM_BACKGROUND_PERSISTENCE
                .add(1, router_env::metric_attributes!(("outcome", "succeeded")));
            logger::info!(
                payment_method_session_id = %tracking_data.payment_method_session_id.get_string_repr(),
                payment_method_id = %tracking_data.payment_method_id.get_string_repr(),
                persisted_payment_method_id = %persisted_payment_method_id.get_string_repr(),
                "Completed durable payment method session confirm persistence"
            );
            db.as_scheduler()
                .finish_process_with_business_status(process, COMPLETED_BY_PT_STATUS)
                .await?;
        }
        Err(error) => {
            metrics::PAYMENT_METHOD_SESSION_CONFIRM_BACKGROUND_PERSISTENCE
                .add(1, router_env::metric_attributes!(("outcome", "failed")));
            logger::error!(
                payment_method_session_id = %tracking_data.payment_method_session_id.get_string_repr(),
                payment_method_id = %tracking_data.payment_method_id.get_string_repr(),
                ?error,
                "Failed durable payment method session confirm persistence"
            );

            let mapping = process_data::PaymentMethodsPTMapping::default();
            let time_delta = if process.retry_count == 0 {
                Some(mapping.default_mapping.start_after)
            } else {
                pt_utils::get_delay(
                    process.retry_count + 1,
                    &mapping.default_mapping.frequencies,
                )
            };

            match pt_utils::get_time_from_delta(time_delta) {
                Some(schedule_time) => {
                    metrics::PAYMENT_METHOD_SESSION_CONFIRM_BACKGROUND_PERSISTENCE.add(
                        1,
                        router_env::metric_attributes!(("outcome", "retry_scheduled")),
                    );
                    db.as_scheduler()
                        .retry_process(process, schedule_time)
                        .await?;
                }
                None => {
                    metrics::PAYMENT_METHOD_SESSION_CONFIRM_BACKGROUND_PERSISTENCE.add(
                        1,
                        router_env::metric_attributes!(("outcome", "retries_exceeded")),
                    );
                    db.as_scheduler()
                        .finish_process_with_business_status(process, RETRIES_EXCEEDED_STATUS)
                        .await?;
                }
            }
        }
    }

    Ok(())
}
