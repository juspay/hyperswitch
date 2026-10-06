use common_utils::ext_traits::ValueExt;
use diesel_models::process_tracker::business_status;
use router_env::logger;
use scheduler::{
    consumer::{self, workflows::ProcessTrackerWorkflow},
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

        let provider_key_store = db
            .get_merchant_key_store_by_merchant_id(
                &tracking_data.merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await?;
        let provider_account = db
            .find_merchant_account_by_merchant_id(&tracking_data.merchant_id, &provider_key_store)
            .await?;

        let processor_merchant_id = tracking_data
            .processor_merchant_id
            .as_ref()
            .unwrap_or(&tracking_data.merchant_id);
        let processor_key_store = db
            .get_merchant_key_store_by_merchant_id(
                processor_merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await?;
        let processor_account = db
            .find_merchant_account_by_merchant_id(processor_merchant_id, &processor_key_store)
            .await?;

        let platform = domain::Platform::new(
            provider_account,
            provider_key_store,
            processor_account,
            processor_key_store,
            None,
        );

        let payment_attempt = get_payment_attempt_from_object_reference_id(
            state,
            tracking_data.dispute_payload.object_reference_id.clone(),
            platform.get_processor(),
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
        let dispute: Option<diesel_models::dispute::Dispute> = state
            .store
            .find_by_processor_merchant_id_payment_id_connector_dispute_id(
                platform.get_processor().get_account().get_id(),
                &payment_attempt.payment_id,
                &tracking_data.dispute_payload.connector_dispute_id,
                platform.get_processor().get_account().storage_scheme,
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
            let payment_id = payment_attempt.payment_id.clone();
            // Update dispute data
            let response = Box::pin(disputes::update_dispute_data(
                state,
                platform,
                business_profile,
                dispute,
                tracking_data.dispute_payload,
                payment_attempt,
                tracking_data.connector_name.as_str(),
            ))
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
                        state.superposition_service.as_ref(),
                        tracking_data.connector_name,
                        tracking_data
                            .processor_merchant_id
                            .unwrap_or_else(|| tracking_data.merchant_id.clone()),
                        Some(payment_id),
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
    superposition_client: &external_services::superposition::SuperpositionClient,
    dimensions: &crate::core::configs::dimension_state::DimensionsWithProcessorMerchantIdAndConnector,
    retry_count: i32,
    payment_id: Option<&common_utils::id_type::PaymentId>,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let mapping = dimensions
        .get_pt_mapping_dispute_sync(db, superposition_client, payment_id)
        .await;
    let time_delta = scheduler_utils::get_schedule_time(mapping, retry_count);

    Ok(scheduler_utils::get_time_from_delta(time_delta))
}

/// Schedule the task for retry
///
/// Returns bool which indicates whether this was the last retry or not
pub async fn retry_sync_task(
    db: &dyn StorageInterface,
    superposition_client: &external_services::superposition::SuperpositionClient,
    connector: String,
    processor_merchant_id: common_utils::id_type::MerchantId,
    payment_id: Option<common_utils::id_type::PaymentId>,
    pt: storage::ProcessTracker,
) -> Result<bool, sch_errors::ProcessTrackerError> {
    let connector_enum = connector
        .parse::<common_enums::connector_enums::Connector>()
        .map_err(|_| sch_errors::ProcessTrackerError::UnexpectedFlow)?;
    let dimensions = crate::core::configs::dimension_state::Dimensions::new()
        .with_processor_merchant_id(processor_merchant_id.into())
        .with_connector(connector_enum);
    let schedule_time = get_sync_process_schedule_time(
        db,
        superposition_client,
        &dimensions,
        pt.retry_count + 1,
        payment_id.as_ref(),
    )
    .await?;

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
