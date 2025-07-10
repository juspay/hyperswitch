#[cfg(feature = "v2")]
use api_models::payments::PaymentsGetIntentRequest;
#[cfg(feature = "v2")]
use common_utils::{
    ext_traits::{StringExt, ValueExt},
    id_type,
};
#[cfg(feature = "v2")]
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentIntentData;
#[cfg(feature = "v2")]
use router_env::logger;
use scheduler::{consumer::workflows::ProcessTrackerWorkflow, errors};
#[cfg(feature = "v2")]
use scheduler::{types::process_data, utils as scheduler_utils};
#[cfg(feature = "v2")]
use storage_impl::errors as storage_errors;

#[cfg(feature = "v2")]
use crate::{
    core::{
        payments,
        revenue_recovery::{self as pcr},
    },
    db::StorageInterface,
    errors::StorageError,
    types::{
        api::{self as api_types},
        domain,
        storage::revenue_recovery as pcr_storage_types,
    },
};
use crate::{routes::SessionState, types::storage};
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
            .parse_value::<pcr_storage_types::RevenueRecoveryWorkflowTrackingData>(
            "PCRWorkflowTrackingData",
        )?;
        let request = PaymentsGetIntentRequest {
            id: tracking_data.global_payment_id.clone(),
        };
        let revenue_recovery_payment_data =
            extract_data_and_perform_action(state, &tracking_data).await?;
        let merchant_context_from_revenue_recovery_payment_data =
            domain::MerchantContext::NormalMerchant(Box::new(domain::Context(
                revenue_recovery_payment_data.merchant_account.clone(),
                revenue_recovery_payment_data.key_store.clone(),
            )));
        let (payment_data, _, _) = payments::payments_intent_operation_core::<
            api_types::PaymentGetIntent,
            _,
            _,
            PaymentIntentData<api_types::PaymentGetIntent>,
        >(
            state,
            state.get_req_state(),
            merchant_context_from_revenue_recovery_payment_data,
            revenue_recovery_payment_data.profile.clone(),
            payments::operations::PaymentGetIntent,
            request,
            tracking_data.global_payment_id.clone(),
            hyperswitch_domain_models::payments::HeaderPayload::default(),
        )
        .await?;

        match process.name.as_deref() {
            Some("EXECUTE_WORKFLOW") => {
                Box::pin(pcr::perform_execute_payment(
                    state,
                    &process,
                    &tracking_data,
                    &revenue_recovery_payment_data,
                    &payment_data.payment_intent,
                ))
                .await
            }
            Some("PSYNC_WORKFLOW") => {
                Box::pin(pcr::perform_payments_sync(
                    state,
                    &process,
                    &tracking_data,
                    &revenue_recovery_payment_data,
                    &payment_data.payment_intent,
                ))
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
    tracking_data: &pcr_storage_types::RevenueRecoveryWorkflowTrackingData,
) -> Result<pcr_storage_types::RevenueRecoveryPaymentData, errors::ProcessTrackerError> {
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

    let billing_mca = db
        .find_merchant_connector_account_by_id(
            key_manager_state,
            &tracking_data.billing_mca_id,
            &key_store,
        )
        .await?;

    let pcr_payment_data = pcr_storage_types::RevenueRecoveryPaymentData {
        merchant_account,
        profile,
        key_store,
        billing_mca,
        retry_algorithm: tracking_data.revenue_recovery_retry,
    };
    Ok(pcr_payment_data)
}

#[cfg(feature = "v2")]
pub(crate) async fn get_schedule_time_to_retry_mit_payments(
    db: &dyn StorageInterface,
    merchant_id: &id_type::MerchantId,
    retry_count: i32,
) -> Option<time::PrimitiveDateTime> {
    let key = "pt_mapping_pcr_retries";
    let result = db
        .find_config_by_key(key)
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("RevenueRecoveryPaymentProcessTrackerMapping")
                .change_context(StorageError::DeserializationFailed)
        });

    let mapping = result.map_or_else(
        |error| {
            if error.current_context().is_db_not_found() {
                logger::debug!("Revenue Recovery retry config `{key}` not found, ignoring");
            } else {
                logger::error!(
                    ?error,
                    "Failed to read Revenue Recovery retry config `{key}`"
                );
            }
            process_data::RevenueRecoveryPaymentProcessTrackerMapping::default()
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
