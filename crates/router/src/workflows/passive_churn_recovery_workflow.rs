use common_utils::ext_traits::{StringExt, ValueExt};
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentIntentData;
use router_env::logger;
use scheduler::{
    consumer::workflows::ProcessTrackerWorkflow, errors, types::process_data,
    utils as scheduler_utils,
};

#[cfg(feature = "v2")]
use crate::{
    core::passive_churn_recovery as pcr, types::storage::passive_churn_recovery as pcr_types,
};
use crate::{
    core::payments,
    db::StorageInterface,
    errors::StorageError,
    routes::SessionState,
    types::{
        api::{self as api_types},
        storage,
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
        match process.name.as_deref() {
            Some("EXECUTE_WORKFLOW") => {
                let db = &*state.store;

                let tracking_data = process
                    .tracking_data
                    .clone()
                    .parse_value::<pcr_types::PCRExecuteWorkflowTrackingData>(
                    "PCRExecuteWorkflowTrackingData",
                )?;

                let key_manager_state = &state.into();
                let pcr_data = extract_data_and_perform_action(&state, process).await?;
                let (payment_data, _, customer) = payments::payments_intent_operation_core::<
                    api_types::PaymentGetIntent,
                    _,
                    _,
                    PaymentIntentData<api_types::PaymentGetIntent>,
                >(
                    &state,
                    state.get_req_state(),
                    pcr_data.merchant_account,
                    pcr_data.profile,
                    pcr_data.key_store,
                    payments::operations::PaymentGetIntent,
                    tracking_data.request,
                    pcr_data.global_payment_id.clone(),
                    hyperswitch_domain_models::payments::HeaderPayload::default(),
                    pcr_data.platform_merchant_account,
                )
                .await?;

                // handle the call connector field once it has been added
                pcr::decide_execute_pcr_workflow(
                    &state,
                    &process,
                    &payment_data.payment_intent,
                    &key_manager_state,
                    &pcr_data.key_store,
                    &pcr_data.merchant_account,
                    &pcr_data.profile,
                )
                .await
            }
            Some("PSYNC_WORKFLOW") => todo!(),

            Some("REVIEW_WORKFLOW") => todo!(),
            _ => Err(errors::ProcessTrackerError::JobNotFound),
        }
    }
}

#[cfg(feature = "v2")]
pub(crate) async fn extract_data_and_perform_action(
    state: &SessionState,
    process: storage::ProcessTracker,
) -> Result<pcr_types::PCRPaymentData, errors::ProcessTrackerError> {
    let db = &state.store;
    let tracking_data = process
        .tracking_data
        .clone()
        .parse_value::<pcr_types::PCRExecuteWorkflowTrackingData>(
            "PCRExecuteWorkflowTrackingData",
        )?;

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
        if let Some(platform_merchant_id) = tracking_data.platform_merchant_id {
            Some(
                db.find_merchant_account_by_merchant_id(
                    key_manager_state,
                    &platform_merchant_id,
                    &key_store,
                )
                .await?,
            )
        } else {
            None
        };

    let global_payment_id = tracking_data.global_payment_id.clone();
    let pcr_payment_data = pcr_types::PCRPaymentData {
        merchant_account,
        profile,
        platform_merchant_account,
        key_store,
        global_payment_id,
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
                logger::debug!("Outgoing webhooks retry config `{key}` not found, ignoring");
            } else {
                logger::error!(
                    ?error,
                    "Failed to read outgoing webhooks retry config `{key}`"
                );
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
) -> pcr::Action {
    let schedule_time =
        get_schedule_time_to_retry_mit_payments(db, &merchant_id, pt.retry_count + 1).await;
    pt.schedule_time = schedule_time;
    match schedule_time {
        Some(_) => pcr::Action::RetryPayment(pt),

        None => pcr::Action::TerminalPaymentStatus,
    }
}
