//! Retries the `payment_attempt.payment_method_id` DB write when it fails after a
//! save-payment-method locker call already succeeded (#12904). Tracking data is IDs only - the
//! vault entry exists by the time this runs, so nothing sensitive needs to be persisted here.

use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

use crate::{
    errors,
    logger::error,
    routes::SessionState,
    types::storage::{
        self, payment_attempt::SavePaymentMethodAttemptUpdateTrackingData, PaymentAttemptUpdate,
    },
};

pub struct SavePaymentMethodAttemptUpdateWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for SavePaymentMethodAttemptUpdateWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        let tracking_data: SavePaymentMethodAttemptUpdateTrackingData = process
            .tracking_data
            .clone()
            .parse_value("SavePaymentMethodAttemptUpdateTrackingData")?;

        let retry_count = process.retry_count;

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
            .find_payment_attempt_by_attempt_id_processor_merchant_id(
                &tracking_data.attempt_id,
                &tracking_data.merchant_id,
                merchant_account.storage_scheme,
                &key_store,
            )
            .await?;

        // If a later attempt (e.g. a manual retry from the merchant) already recorded a
        // payment_method_id, this task's write would be stale - nothing left to reconcile.
        if payment_attempt.payment_method_id.is_some() {
            return db
                .as_scheduler()
                .finish_process_with_business_status(process, "PROCESS_ALREADY_COMPLETED")
                .await
                .map_err(Into::<errors::ProcessTrackerError>::into);
        }

        let payment_attempt_update = PaymentAttemptUpdate::PaymentMethodDetailsUpdate {
            payment_method_id: tracking_data.payment_method_id.clone(),
            updated_by: tracking_data.updated_by.clone(),
        };

        let res = db
            .update_payment_attempt_with_attempt_id(
                payment_attempt,
                payment_attempt_update,
                merchant_account.storage_scheme,
                &key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to update payment attempt with payment_method_id");

        if res.is_ok() {
            db.as_scheduler()
                .finish_process_with_business_status(process, "COMPLETED_BY_PT")
                .await?;
        } else {
            let mapping = process_data::PaymentMethodsPTMapping::default();
            let time_delta = if retry_count == 0 {
                Some(mapping.default_mapping.start_after)
            } else {
                pt_utils::get_delay(retry_count + 1, &mapping.default_mapping.frequencies)
            };

            let schedule_time = pt_utils::get_time_from_delta(time_delta);

            match schedule_time {
                Some(s_time) => db
                    .as_scheduler()
                    .retry_process(process, s_time)
                    .await
                    .map_err(Into::<errors::ProcessTrackerError>::into)?,
                None => {
                    error!(
                        attempt_id = %tracking_data.attempt_id,
                        "save-payment-method attempt update retries exhausted; \
                         vault holds payment_method_id={:?} but payment_attempt.payment_method_id \
                         was never persisted - needs manual reconciliation",
                        tracking_data.payment_method_id,
                    );
                    db.as_scheduler()
                        .finish_process_with_business_status(process, "RETRIES_EXCEEDED")
                        .await
                        .map_err(Into::<errors::ProcessTrackerError>::into)?
                }
            };
        }

        Ok(())
    }

    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        todo!()
    }

    async fn error_handler<'a>(
        &'a self,
        _state: &'a SessionState,
        process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        error!(%process.id, "Failed while executing save-payment-method attempt update workflow");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use common_utils::{ext_traits::Encode, id_type};

    use super::*;

    #[test]
    fn test_tracking_data_round_trips_through_json() {
        let tracking_data = SavePaymentMethodAttemptUpdateTrackingData {
            attempt_id: "attempt_123".to_string(),
            payment_id: id_type::PaymentId::wrap("payment_123".to_string())
                .expect("valid payment_id"),
            merchant_id: id_type::MerchantId::try_from(std::borrow::Cow::Borrowed("merchant_123"))
                .expect("valid merchant_id"),
            payment_method_id: Some("pm_123".to_string()),
            updated_by: "psql".to_string(),
        };

        let encoded = tracking_data
            .encode_to_value()
            .expect("tracking data must serialize");
        let decoded: SavePaymentMethodAttemptUpdateTrackingData =
            serde_json::from_value(encoded).expect("tracking data must deserialize");

        assert_eq!(decoded.attempt_id, tracking_data.attempt_id);
        assert_eq!(decoded.payment_method_id, tracking_data.payment_method_id);
        assert_eq!(decoded.updated_by, tracking_data.updated_by);
    }

    #[test]
    fn test_default_retry_mapping_matches_payment_method_status_update() {
        let mapping = process_data::PaymentMethodsPTMapping::default();
        assert_eq!(mapping.default_mapping.start_after, 900);
        assert_eq!(mapping.max_retries_count, 5);
    }
}
