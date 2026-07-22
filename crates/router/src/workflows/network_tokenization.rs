#[cfg(feature = "v1")]
use common_utils::ext_traits::ValueExt;
#[cfg(feature = "v1")]
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

use crate::{errors, logger, routes::SessionState, types::storage};

pub struct NetworkTokenizationWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for NetworkTokenizationWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        use crate::{
            core::payment_methods::network_tokenization,
            types::storage::NetworkTokenizationTrackingData,
        };

        let db = &*state.store;

        let tracking_data: NetworkTokenizationTrackingData = process
            .tracking_data
            .clone()
            .parse_value("NetworkTokenizationTrackingData")?;

        let retry_count = process.retry_count;
        let merchant_id = tracking_data.merchant_id.clone();

        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await?;

        let business_profile = db
            .find_business_profile_by_merchant_id_profile_id(
                &key_store,
                &merchant_id,
                &merchant_account
                    .default_profile
                    .clone()
                    .ok_or(errors::ProcessTrackerError::EApiErrorResponse)?,
            )
            .await?;

        if !business_profile.is_network_tokenization_enabled {
            logger::info!(
                payment_method_id=%tracking_data.payment_method_id,
                "Network tokenization is disabled for this profile, skipping"
            );
            return db
                .as_scheduler()
                .finish_process_with_business_status(process, "SKIPPED_NT_DISABLED")
                .await
                .map_err(Into::<errors::ProcessTrackerError>::into);
        }

        let payment_method = db
            .find_payment_method(
                &key_store,
                &tracking_data.payment_method_id,
                merchant_account.storage_scheme,
            )
            .await?;

        // Skip if already tokenized
        if payment_method
            .network_token_requestor_reference_id
            .is_some()
        {
            logger::info!(
                payment_method_id=%tracking_data.payment_method_id,
                "Payment method already has a network token, skipping"
            );
            return db
                .as_scheduler()
                .finish_process_with_business_status(process, "ALREADY_TOKENIZED")
                .await
                .map_err(Into::<errors::ProcessTrackerError>::into);
        }

        let platform = crate::types::domain::Platform::new(
            merchant_account.clone(),
            key_store.clone(),
            merchant_account,
            key_store,
            None,
        );

        let result = network_tokenization::generate_network_token_for_payment_method(
            state,
            &platform,
            &tracking_data,
            payment_method,
        )
        .await;

        match result {
            Ok(()) => {
                db.as_scheduler()
                    .finish_process_with_business_status(process, "COMPLETED_BY_PT")
                    .await
                    .map_err(Into::<errors::ProcessTrackerError>::into)?;
            }
            Err(err) => {
                logger::error!(
                    payment_method_id=%tracking_data.payment_method_id,
                    ?err,
                    "Failed to generate network token in process tracker workflow"
                );
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
                    None => db
                        .as_scheduler()
                        .finish_process_with_business_status(process, "RETRIES_EXCEEDED")
                        .await
                        .map_err(Into::<errors::ProcessTrackerError>::into)?,
                };
            }
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
        logger::error!(%process.id, "Failed while executing NetworkTokenizationWorkflow");
        Ok(())
    }
}
