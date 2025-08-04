use common_utils::ext_traits::{StringExt, ValueExt};
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use router_env::{logger, tracing::Instrument};
use scheduler::{
    consumer::{self, types::process_data, workflows::ProcessTrackerWorkflow},
    errors as sch_errors, utils as scheduler_utils,
};
use std::ops::Deref;

use crate::{
    core::{disputes},
    db::StorageInterface,
    errors,
    routes::SessionState,
    types::{api, domain, storage},
};

pub struct DisputeListSyncWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for DisputeListSyncWorkflow {
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
        let key_manager_state = &state.into();
        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &tracking_data.merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(
                key_manager_state,
                &tracking_data.merchant_id.clone(),
                &key_store,
            )
            .await?;

        let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(domain::Context(
            merchant_account.clone(),
            key_store.clone(),
        )));


        let business_profile = state
            .store
            .find_business_profile_by_profile_id(
                &(state).into(),
                merchant_context.get_merchant_key_store(),
                &tracking_data.profile_id,
            )
            .await?;

            let offset_date_time = time::OffsetDateTime::now_utc(); 
            let current_time = time::PrimitiveDateTime::new(offset_date_time.date(), offset_date_time.time());
            let dispute_polling_interval = business_profile.dispute_polling_interval.map(|dispute_polling_interval| *dispute_polling_interval.deref()).unwrap_or(24);
            let schedule_time = current_time
    .checked_add(time::Duration::hours(dispute_polling_interval as i64))
    .ok_or(sch_errors::ProcessTrackerError::TypeConversionError)?;

        disputes::add_dispute_list_sync_task_to_pt(
                        db,
                        &tracking_data.connector_name,
                        tracking_data.merchant_id.clone(),
                        tracking_data.merchant_connector_id.clone(),
                        schedule_time,
                       current_time,
                       tracking_data.profile_id).await?;
        let req = hyperswitch_domain_models::router_request_types::FetchDisputesRequestData {
            created_from: tracking_data.created_from,
            created_to: Some(current_time)
        };

        let response = disputes::fetch_disputes_from_connector(
            state.clone(),
            merchant_context,
            tracking_data.merchant_connector_id,
            req,
        )
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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_get_default_schedule_time() {
        let merchant_id =
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::from("-")).unwrap();
        let schedule_time_delta = scheduler_utils::get_schedule_time(
            process_data::ConnectorPTMapping::default(),
            &merchant_id,
            0,
        )
        .unwrap();
        let first_retry_time_delta = scheduler_utils::get_schedule_time(
            process_data::ConnectorPTMapping::default(),
            &merchant_id,
            1,
        )
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
