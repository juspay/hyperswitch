use common_utils::ext_traits::{StringExt, ValueExt};
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use router_env::logger;
use scheduler::{
    consumer::{self, types::process_data, workflows::ProcessTrackerWorkflow},
    errors as sch_errors, utils as scheduler_utils,
};

#[cfg(feature = "v1")]
use crate::core::webhooks::incoming::get_payment_attempt_from_object_reference_id;
use crate::{
    core::disputes,
    db::StorageInterface,
    errors,
    routes::SessionState,
    types::{api, domain, storage},
};

pub struct ProcessDisputeWorkflow;

/// This workflow inserts only new dispute records into the dispute table and triggers related outgoing webhook
#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for ProcessDisputeWorkflow {
    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), sch_errors::ProcessTrackerError> {
        todo!()
    }

    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), sch_errors::ProcessTrackerError> {
        let db: &dyn StorageInterface = &*state.store;
        let tracking_data: api::ProcessDisputePTData = process
            .tracking_data
            .clone()
            .parse_value("ProcessDisputePTData")?;
        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                &tracking_data.merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &key_store)
            .await?;

        let platform = domain::Platform::new(
            merchant_account.clone(),
            key_store.clone(),
            merchant_account,
            key_store,
        );

        let payment_attempt = get_payment_attempt_from_object_reference_id(
            state,
            tracking_data.dispute_payload.object_reference_id.clone(),
            &platform,
        )
        .await?;

        let business_profile = state
            .store
            .find_business_profile_by_profile_id(
                platform.get_processor().get_key_store(),
                &payment_attempt.profile_id,
            )
            .await?;

        // Check if the dispute already exists
        let dispute = state
            .store
            .find_by_merchant_id_payment_id_connector_dispute_id(
                platform.get_processor().get_account().get_id(),
                &payment_attempt.payment_id,
                &tracking_data.dispute_payload.connector_dispute_id,
            )
            .await
            .ok()
            .flatten();

        if dispute.is_some() {
            // Dispute already exists — mark the process as complete
            state
                .store
                .as_scheduler()
                .finish_process_with_business_status(process, business_status::COMPLETED_BY_PT)
                .await?;
        } else {
            // Update dispute data
            let response = disputes::update_dispute_data(
                state,
                platform,
                business_profile,
                dispute,
                tracking_data.dispute_payload,
                payment_attempt,
                tracking_data.connector_name.as_str(),
            )
            .await
            .map_err(|error| logger::error!("Dispute update failed: {error}"));

            match response {
                Ok(_) => {
                    state
                        .store
                        .as_scheduler()
                        .finish_process_with_business_status(
                            process,
                            business_status::COMPLETED_BY_PT,
                        )
                        .await?;
                }
                Err(_) => {
                    retry_sync_task(
                        db,
                        tracking_data.connector_name,
                        tracking_data.merchant_id,
                        process,
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: sch_errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), sch_errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}

pub async fn get_sync_process_schedule_time(
    db: &dyn StorageInterface,
    connector: &str,
    merchant_id: &common_utils::id_type::MerchantId,
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
    merchant_id: common_utils::id_type::MerchantId,
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
                .finish_process_with_business_status(pt, business_status::RETRIES_EXCEEDED)
                .await?;
            Ok(true)
        }
    }
}
