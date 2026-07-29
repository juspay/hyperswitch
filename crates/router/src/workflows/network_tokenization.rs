use common_utils::ext_traits::ValueExt;
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

#[cfg(feature = "v2")]
use crate::core::payment_methods;
#[cfg(feature = "v1")]
use crate::core::payments::tokenization;
use crate::{
    errors, logger,
    routes::SessionState,
    types::{domain, storage, storage::NetworkTokenizationTrackingData},
};

const NETWORK_TOKENIZATION_DISABLED_STATUS: &str = "SKIPPED_NT_DISABLED";
const ALREADY_TOKENIZED_STATUS: &str = "ALREADY_TOKENIZED";
const COMPLETED_BY_PT_STATUS: &str = "COMPLETED_BY_PT";
const RETRIES_EXCEEDED_STATUS: &str = "RETRIES_EXCEEDED";

pub struct NetworkTokenizationWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for NetworkTokenizationWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
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
                &tracking_data.profile_id,
            )
            .await?;

        if !business_profile.is_network_tokenization_enabled {
            // A skip is a terminal success, not a failure, so finish with a business status
            // rather than mapping it to an error state (which would trigger retries).
            logger::info!(
                payment_method_id=%tracking_data.payment_method_id,
                "Network tokenization is disabled for this profile, skipping"
            );
            db.as_scheduler()
                .finish_process_with_business_status(process, NETWORK_TOKENIZATION_DISABLED_STATUS)
                .await?;
        } else {
            let payment_method = db
                .find_payment_method(
                    &key_store,
                    &tracking_data.payment_method_id,
                    merchant_account.storage_scheme,
                )
                .await?;

            if payment_method
                .network_token_requestor_reference_id
                .is_some()
            {
                // Already tokenized — another terminal success, no retry needed.
                logger::info!(
                    payment_method_id=%tracking_data.payment_method_id,
                    "Payment method already has a network token, skipping"
                );
                db.as_scheduler()
                    .finish_process_with_business_status(process, ALREADY_TOKENIZED_STATUS)
                    .await?;
            } else {
                let platform = domain::Platform::new(
                    merchant_account.clone(),
                    key_store.clone(),
                    merchant_account,
                    key_store,
                    None,
                );

                let result = Box::pin(tokenization::generate_network_token_for_payment_method(
                    state,
                    &platform,
                    &tracking_data,
                    payment_method,
                ))
                .await;

                match result {
                    Ok(()) => {
                        db.as_scheduler()
                            .finish_process_with_business_status(process, COMPLETED_BY_PT_STATUS)
                            .await?;
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
                            pt_utils::get_delay(
                                retry_count + 1,
                                &mapping.default_mapping.frequencies,
                            )
                        };
                        match pt_utils::get_time_from_delta(time_delta) {
                            Some(s_time) => {
                                db.as_scheduler().retry_process(process, s_time).await?
                            }
                            None => {
                                db.as_scheduler()
                                    .finish_process_with_business_status(
                                        process,
                                        RETRIES_EXCEEDED_STATUS,
                                    )
                                    .await?
                            }
                        };
                    }
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
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
                &tracking_data.profile_id,
            )
            .await?;

        if !business_profile.is_network_tokenization_enabled {
            // A skip is a terminal success, not a failure, so finish with a business status
            // rather than mapping it to an error state (which would trigger retries).
            logger::info!(
                payment_method_id=%tracking_data.payment_method_id.get_string_repr(),
                "Network tokenization is disabled for this profile, skipping"
            );
            db.as_scheduler()
                .finish_process_with_business_status(process, NETWORK_TOKENIZATION_DISABLED_STATUS)
                .await?;
        } else {
            let payment_method = db
                .find_payment_method(
                    &key_store,
                    &tracking_data.payment_method_id,
                    merchant_account.storage_scheme,
                )
                .await?;

            if payment_method
                .network_token_requestor_reference_id
                .is_some()
            {
                // Already tokenized — another terminal success, no retry needed.
                logger::info!(
                    payment_method_id=%tracking_data.payment_method_id.get_string_repr(),
                    "Payment method already has a network token, skipping"
                );
                db.as_scheduler()
                    .finish_process_with_business_status(process, ALREADY_TOKENIZED_STATUS)
                    .await?;
            } else {
                let platform = domain::Platform::new(
                    merchant_account.clone(),
                    key_store.clone(),
                    merchant_account,
                    key_store,
                    None,
                );

                let result = Box::pin(payment_methods::generate_network_token_for_payment_method(
                    state,
                    &platform,
                    &business_profile,
                    payment_method,
                ))
                .await;

                match result {
                    Ok(()) => {
                        db.as_scheduler()
                            .finish_process_with_business_status(process, COMPLETED_BY_PT_STATUS)
                            .await?;
                    }
                    Err(err) => {
                        logger::error!(
                            payment_method_id=%tracking_data.payment_method_id.get_string_repr(),
                            ?err,
                            "Failed to generate network token in process tracker workflow"
                        );
                        let mapping = process_data::PaymentMethodsPTMapping::default();
                        let time_delta = if retry_count == 0 {
                            Some(mapping.default_mapping.start_after)
                        } else {
                            pt_utils::get_delay(
                                retry_count + 1,
                                &mapping.default_mapping.frequencies,
                            )
                        };
                        match pt_utils::get_time_from_delta(time_delta) {
                            Some(s_time) => {
                                db.as_scheduler().retry_process(process, s_time).await?
                            }
                            None => {
                                db.as_scheduler()
                                    .finish_process_with_business_status(
                                        process,
                                        RETRIES_EXCEEDED_STATUS,
                                    )
                                    .await?
                            }
                        };
                    }
                }
            }
        }

        Ok(())
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
