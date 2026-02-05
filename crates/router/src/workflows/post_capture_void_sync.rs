#[cfg(feature = "v2")]
use common_utils::ext_traits::AsyncExt;
use common_utils::ext_traits::{OptionExt, StringExt, ValueExt};
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use router_env::logger;
use scheduler::{
    consumer::{self, types::process_data, workflows::ProcessTrackerWorkflow},
    errors as sch_errors, utils as scheduler_utils,
};

#[cfg(feature = "v2")]
use crate::workflows::revenue_recovery::update_token_expiry_based_on_schedule_time;
use crate::{
    consts,
    core::{
        errors::StorageErrorExt,
        payments::{self as payment_flows, operations},
    },
    db::StorageInterface,
    errors,
    routes::SessionState,
    services,
    types::{
        api, domain,
        storage::{self, enums},
    },
    utils,
};

pub struct PaymentsPostCaptureVoidSyncWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentsPostCaptureVoidSyncWorkflow {
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
        let tracking_data: api::PaymentsCancelPostCaptureSyncBody = process
            .tracking_data
            .clone()
            .parse_value("PaymentsCancelPostCaptureSyncBody")?;
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

        let platform = domain::Platform::new(
            merchant_account.clone(),
            key_store.clone(),
            merchant_account.clone(),
            key_store.clone(),
            None,
        );
        // TODO: Add support for ReqState in PT flows
        let (mut payment_data, _, _, _) = Box::pin(payment_flows::payments_operation_core::<
            api::PostCaptureVoidSync,
            _,
            _,
            _,
            payment_flows::PaymentData<api::PostCaptureVoidSync>,
        >(
            state,
            state.get_req_state(),
            &platform,
            None,
            operations::PaymentCancelPostCaptureSync,
            tracking_data.clone(),
            payment_flows::CallConnectorAction::Trigger,
            None,
            services::AuthFlow::Client,
            None,
            hyperswitch_domain_models::payments::HeaderPayload::default(),
        ))
        .await?;

        let terminal_status = [
            enums::AttemptStatus::RouterDeclined,
            enums::AttemptStatus::AutoRefunded,
            enums::AttemptStatus::Voided,
            enums::AttemptStatus::CaptureFailed,
            enums::AttemptStatus::Failure,
        ];

        let is_post_capture_void_attempted_state = payment_data.payment_intent.is_post_capture_void_applied() || 

        match &payment_data.payment_attempt.status {
    status
        if terminal_status.contains(status)
            || !payment_data
                .payment_intent
                .is_post_capture_void_pending() =>
    {
        state
            .store
            .as_scheduler()
            .finish_process_with_business_status(
                process,
                business_status::COMPLETED_BY_PT,
            )
            .await?;
    }
    _ => {retry_sync_task(
                    db,
                    connector,
                    payment_data.payment_attempt.merchant_id.clone(),
                    process,
                )
                .await?}
};Ok(())
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

pub async fn get_post_capture_void_sync_process_schedule_time(
    db: &dyn StorageInterface,
    connector: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    let mapping: common_utils::errors::CustomResult<
        process_data::ConnectorPTMapping,
        errors::StorageError,
    > = db
        .find_config_by_key(&format!("pt_mapping_post_capture_void_sync_{connector}"))
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