use api_models::payments::PaymentsGetIntentRequest;
#[cfg(feature = "v2")]
use common_utils::ext_traits::{StringExt, ValueExt};
#[cfg(feature = "v2")]
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentIntentData;
#[cfg(feature = "v2")]
use router_env::logger;
#[cfg(feature = "v2")]
use scheduler::{
    consumer::workflows::ProcessTrackerWorkflow, errors, types::process_data,
    utils as scheduler_utils,
};

#[cfg(feature = "v2")]
use crate::{
    core::{
        passive_churn_recovery::{self as pcr, types as pcr_types},
        payments,
    },
    db::StorageInterface,
    errors::StorageError,
    routes::SessionState,
    types::{
        api::{self as api_types},
        storage::{self, passive_churn_recovery as pcr_storage_types},
    },
};

pub struct ExecutePcrWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for ExecutePcrWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Ok(())
    }
    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data = process
            .tracking_data
            .clone()
            .parse_value::<pcr_storage_types::PCRWorkflowTrackingData>(
            "PCRWorkflowTrackingData",
        )?;
        let request = PaymentsGetIntentRequest {
            id: tracking_data.global_payment_id.clone(),
        };
        let key_manager_state = &state.into();
        let pcr_data = extract_data_and_perform_action(state, &tracking_data).await?;
        let (payment_data, _, _) = payments::payments_intent_operation_core::<
            api_types::PaymentGetIntent,
            _,
            _,
            PaymentIntentData<api_types::PaymentGetIntent>,
        >(
            state,
            state.get_req_state(),
            pcr_data.merchant_account.clone(),
            pcr_data.profile.clone(),
            pcr_data.key_store.clone(),
            payments::operations::PaymentGetIntent,
            request,
            tracking_data.global_payment_id.clone(),
            hyperswitch_domain_models::payments::HeaderPayload::default(),
            pcr_data.platform_merchant_account.clone(),
        )
        .await?;

        match process.name.as_deref() {
            Some("EXECUTE_WORKFLOW") => {
                // handle the call connector field once it has been added
                pcr::decide_execute_pcr_workflow(
                    state,
                    &process,
                    &tracking_data,
                    &pcr_data,
                    key_manager_state,
                    &payment_data.payment_intent,
                )
                .await
            }
            Some("PSYNC_WORKFLOW") => {
                Box::pin(pcr::decide_execute_psync_workflow(
                    state,
                    &process,
                    &tracking_data,
                    &pcr_data,
                    key_manager_state,
                    &payment_data.payment_intent,
                ))
                .await?;
                Ok(())
            }

            Some("REVIEW_WORKFLOW") => {
                pcr::review_workflow(
                    state,
                    &process,
                    &tracking_data,
                    &pcr_data,
                    key_manager_state,
                    &payment_data.payment_intent,
                )
                .await?;
                Ok(())
            }
            _ => Err(errors::ProcessTrackerError::JobNotFound),
        }
    }
}

#[cfg(feature = "v2")]
pub(crate) async fn extract_data_and_perform_action(
    state: &SessionState,
    tracking_data: &pcr_storage_types::PCRWorkflowTrackingData,
) -> Result<pcr_storage_types::PCRPaymentData, errors::ProcessTrackerError> {
    let db = &state.store;

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
            &tracking_data.merchant_id,
            &key_store,
        )
        .await?;

    let profile = db
        .find_business_profile_by_profile_id(
            key_manager_state,
            &key_store,
            &tracking_data.profile_id,
        )
        .await?;
    let platform_merchant_account =
        if let Some(platform_merchant_id) = &tracking_data.platform_merchant_id {
            Some(
                db.find_merchant_account_by_merchant_id(
                    key_manager_state,
                    platform_merchant_id,
                    &key_store,
                )
                .await?,
            )
        } else {
            None
        };

    let pcr_payment_data = pcr_storage_types::PCRPaymentData {
        merchant_account,
        profile,
        platform_merchant_account,
        key_store,
    };
    Ok(pcr_payment_data)
}

#[cfg(feature = "v2")]
pub(crate) async fn get_schedule_time_to_retry_mit_payments(
    db: &dyn StorageInterface,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_count: i32,
) -> Option<time::PrimitiveDateTime> {
    let key = "pt_mapping_pcr_retries";
    let result = db
        .find_config_by_key(key)
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("PCRPaymentRetryProcessTrackerMapping")
                .change_context(StorageError::DeserializationFailed)
        });

    let mapping = result.map_or_else(
        |error| {
            if error.current_context().is_db_not_found() {
                logger::debug!("PCR retry config `{key}` not found, ignoring");
            } else {
                logger::error!(?error, "Failed to read PCR retry config `{key}`");
            }
            process_data::PCRPaymentRetryProcessTrackerMapping::default()
        },
        |mapping| {
            logger::debug!(?mapping, "Using custom pcr payments retry config");
            mapping
        },
    );

    let time_delta =
        scheduler_utils::get_pcr_payments_retry_schedule_time(mapping, merchant_id, retry_count);

    scheduler_utils::get_time_from_delta(time_delta)
}

#[cfg(feature = "v2")]
pub(crate) async fn retry_pcr_payment_task(
    db: &dyn StorageInterface,
    merchant_id: common_utils::id_type::MerchantId,
    mut pt: storage::ProcessTracker,
) -> pcr_types::Action {
    let schedule_time =
        get_schedule_time_to_retry_mit_payments(db, &merchant_id, pt.retry_count + 1).await;
    pt.schedule_time = schedule_time;
    match schedule_time {
        Some(_) => pcr_types::Action::RetryPayment(pt),

        None => pcr_types::Action::TerminalFailure,
    }
}
