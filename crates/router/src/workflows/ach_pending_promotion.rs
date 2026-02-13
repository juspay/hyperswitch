use common_utils::ext_traits::ValueExt;
use diesel_models::process_tracker::business_status;
use router_env::logger;
use scheduler::{consumer::workflows::ProcessTrackerWorkflow, errors as sch_errors};

use crate::{
    core::errors::StorageErrorExt,
    db::StorageInterface,
    errors,
    routes::SessionState,
    types::storage::{self, enums},
};

/// Workflow that auto-promotes Payload ACH payments from Pending to Charged
/// after the 3-day pending window has elapsed without a reject/decline webhook.
pub struct AchPendingPromotionWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for AchPendingPromotionWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), sch_errors::ProcessTrackerError> {
        let db: &dyn StorageInterface = &*state.store;

        let tracking_data: storage::AchPendingPromotionTrackingData = process
            .tracking_data
            .clone()
            .parse_value("AchPendingPromotionTrackingData")?;

        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                &tracking_data.merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &key_store)
            .await?;

        let payment_attempt = db
            .find_payment_attempt_by_payment_id_processor_merchant_id_attempt_id(
                &tracking_data.payment_id,
                &tracking_data.merchant_id,
                &tracking_data.attempt_id,
                merchant_account.storage_scheme,
                &key_store,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // Only promote if the payment is still in Pending status.
        // If it has already moved to Failure (via a reject/decline webhook) or any other
        // terminal state, we should not override it.
        if payment_attempt.status != enums::AttemptStatus::Pending {
            logger::info!(
                "ACH pending promotion skipped for attempt {}: status is already {:?}",
                tracking_data.attempt_id,
                payment_attempt.status
            );
            return db
                .as_scheduler()
                .finish_process_with_business_status(process, "PROCESS_ALREADY_COMPLETED")
                .await
                .map_err(Into::<sch_errors::ProcessTrackerError>::into);
        }

        // Promote the payment attempt from Pending to Charged
        let payment_attempt_update =
            hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptUpdate::StatusUpdate
            {
                status: enums::AttemptStatus::Charged,
                updated_by: merchant_account.storage_scheme.to_string(),
            };

        db.update_payment_attempt_with_attempt_id(
            payment_attempt,
            payment_attempt_update,
            merchant_account.storage_scheme,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        // Update the payment intent status to Succeeded
        let payment_intent = db
            .find_payment_intent_by_payment_id_processor_merchant_id(
                &tracking_data.payment_id,
                &tracking_data.merchant_id,
                &key_store,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let payment_intent_update =
            hyperswitch_domain_models::payments::payment_intent::PaymentIntentUpdate::PGStatusUpdate
            {
                status: enums::IntentStatus::Succeeded,
                updated_by: merchant_account.storage_scheme.to_string(),
                incremental_authorization_allowed: Some(false),
                feature_metadata: payment_intent.feature_metadata.clone().map(masking::Secret::new),
            };

        db.update_payment_intent(
            payment_intent,
            payment_intent_update,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        logger::info!(
            "ACH pending promotion completed: attempt {} promoted to Charged",
            tracking_data.attempt_id
        );

        db.as_scheduler()
            .finish_process_with_business_status(process, business_status::COMPLETED_BY_PT)
            .await?;

        Ok(())
    }

    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), sch_errors::ProcessTrackerError> {
        todo!()
    }

    async fn error_handler<'a>(
        &'a self,
        _state: &'a SessionState,
        process: storage::ProcessTracker,
        _error: sch_errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), sch_errors::ProcessTrackerError> {
        logger::error!(%process.id, "Failed while executing ACH pending promotion workflow");
        Ok(())
    }
}
