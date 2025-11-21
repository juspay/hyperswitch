use std::ops::Deref;

use common_utils::ext_traits::{StringExt, ValueExt};
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use router_env::{logger, tracing::Instrument};
use scheduler::{
    consumer::{self, types::process_data, workflows::ProcessTrackerWorkflow},
    errors as sch_errors, utils as scheduler_utils,
};

use crate::{
    core::disputes,
    db::StorageInterface,
    errors,
    routes::SessionState,
    types::{api, domain, storage},
};

pub struct DisputeListWorkflow;

/// This workflow fetches disputes from the connector for a given time range
/// and creates a process tracker task for each dispute.
/// It also schedules the next dispute list sync after dispute_polling_hours.
#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for DisputeListWorkflow {
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
        let db = &*state.store;
        let tracking_data: api::DisputeListPTData = process
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
            .find_merchant_account_by_merchant_id(&tracking_data.merchant_id.clone(), &key_store)
            .await?;

        let platform = domain::Platform::new(
            merchant_account.clone(),
            key_store.clone(),
            merchant_account,
            key_store,
        );

        let business_profile = state
            .store
            .find_business_profile_by_profile_id(
                platform.get_processor().get_key_store(),
                &tracking_data.profile_id,
            )
            .await?;

        if process.retry_count == 0 {
            let m_db = state.clone().store;
            let m_tracking_data = tracking_data.clone();
            let dispute_polling_interval = *business_profile
                .dispute_polling_interval
                .unwrap_or_default()
                .deref();

            tokio::spawn(
                async move {
                    schedule_next_dispute_list_task(
                        &*m_db,
                        &m_tracking_data,
                        dispute_polling_interval,
                    )
                    .await
                    .map_err(|error| {
                        logger::error!(
                            "Failed to add dispute list task to process tracker: {error}"
                        )
                    })
                }
                .in_current_span(),
            );
        };

        let response = Box::pin(disputes::fetch_disputes_from_connector(
            state.clone(),
            platform,
            tracking_data.merchant_connector_id,
            hyperswitch_domain_models::router_request_types::FetchDisputesRequestData {
                created_from: tracking_data.created_from,
                created_till: tracking_data.created_till,
            },
        ))
        .await
        .attach_printable("Dispute update failed");

        if response.is_err() {
            retry_sync_task(
                db,
                tracking_data.connector_name,
                tracking_data.merchant_id,
                process,
            )
            .await?;
        } else {
            state
                .store
                .as_scheduler()
                .finish_process_with_business_status(process, business_status::COMPLETED_BY_PT)
                .await?
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
    let schedule_time: Option<time::PrimitiveDateTime> =
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

#[cfg(feature = "v1")]
pub async fn schedule_next_dispute_list_task(
    db: &dyn StorageInterface,
    tracking_data: &api::DisputeListPTData,
    dispute_polling_interval: i32,
) -> Result<(), errors::ProcessTrackerError> {
    let new_created_till = tracking_data
        .created_till
        .checked_add(time::Duration::hours(i64::from(dispute_polling_interval)))
        .ok_or(sch_errors::ProcessTrackerError::TypeConversionError)?;

    let fetch_request = hyperswitch_domain_models::router_request_types::FetchDisputesRequestData {
        created_from: tracking_data.created_till,
        created_till: new_created_till,
    };

    disputes::add_dispute_list_task_to_pt(
        db,
        &tracking_data.connector_name,
        tracking_data.merchant_id.clone(),
        tracking_data.merchant_connector_id.clone(),
        tracking_data.profile_id.clone(),
        fetch_request,
    )
    .await?;
    Ok(())
}
