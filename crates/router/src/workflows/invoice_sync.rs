use api_models::process_tracker as process_tracker_types;
use async_trait::async_trait;
use common_utils::{
    errors::CustomResult,
    ext_traits::{StringExt, ValueExt},
};
use diesel_models::process_tracker::business_status;
use error_stack::ResultExt;
use router_env::logger;
use scheduler::{
    consumer::{self, workflows::ProcessTrackerWorkflow},
    errors,
    types::process_data,
    utils as scheduler_utils,
};

#[cfg(feature = "v1")]
use crate::core::subscription;
use crate::{
    db::StorageInterface,
    routes::SessionState,
    types::{domain, storage},
};

const IVOICE_SYNC_WORKFLOW: &str = "INVOICE_SYNC";
const IVOICE_SYNC_WORKFLOW_TAG: &str = "INVOICE";
pub struct InvoiceSyncWorkflow;

#[async_trait]
impl ProcessTrackerWorkflow<SessionState> for InvoiceSyncWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data = process
            .tracking_data
            .clone()
            .parse_value::<api_models::process_tracker::invoice_sync::InvoiceSyncTrackingData>(
            "InvoiceSyncTrackingData",
        )?;

        match process.name.as_deref() {
            Some(IVOICE_SYNC_WORKFLOW) => {
                Box::pin(perform_subscription_invoice_sync(
                    state,
                    process,
                    &tracking_data,
                ))
                .await
            }
            _ => Err(errors::ProcessTrackerError::JobNotFound),
        }
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> CustomResult<(), errors::ProcessTrackerError> {
        logger::error!("Encountered error");
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }

    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        todo!()
    }
}
#[cfg(feature = "v1")]
async fn perform_subscription_invoice_sync(
    state: &SessionState,
    process: storage::ProcessTracker,
    tracking_data: &api_models::process_tracker::invoice_sync::InvoiceSyncTrackingData,
) -> Result<(), errors::ProcessTrackerError> {
    // Extract merchant context
    let key_manager_state = &state.into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &tracking_data.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(
            key_manager_state,
            &tracking_data.merchant_id,
            &key_store,
        )
        .await?;

    let billing_processor_mca = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            key_manager_state,
            merchant_account.get_id(),
            &tracking_data.billing_processor_mca_id,
            &key_store,
        )
        .await?;

    // Call Payemnt Sync API
    // If payment is successful, record back to billing processor
    // else if pending, schedule a retry

    let status = common_enums::IntentStatus::Succeeded;

    if status == common_enums::IntentStatus::Succeeded {
        // Record back to billing processor
        perform_billing_processor_record_back(state, &merchant_account, &key_store, tracking_data)
            .await
            .attach_printable("Failed to record back to billing processor")?;

        state
            .store
            .as_scheduler()
            .finish_process_with_business_status(process.clone(), business_status::COMPLETED_BY_PT)
            .await?
    } else if status == common_enums::IntentStatus::Processing {
        let db = &*state.store;
        let connector = billing_processor_mca.connector_name.clone();
        let is_last_retry = retry_subscription_invoice_sync_task(
            db,
            connector,
            merchant_account.get_id().to_owned(),
            process.clone(),
        )
        .await?;

        if is_last_retry {
            state
                .store
                .as_scheduler()
                .finish_process_with_business_status(process, business_status::COMPLETED_BY_PT)
                .await?
        }
    } else {
        // Handle payment failure - log the payment status and return appropriate error
        logger::error!(
            "Payment failed for invoice record back. Payment ID: {:?}, Status: {:?}",
            tracking_data.payment_id,
            status
        );
        return Err(errors::ProcessTrackerError::FlowExecutionError {
            flow: IVOICE_SYNC_WORKFLOW,
        });
    }

    Ok(())
}

#[cfg(feature = "v1")]
pub async fn perform_billing_processor_record_back(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    tracking_data: &process_tracker_types::invoice_sync::InvoiceSyncTrackingData,
) -> CustomResult<(), crate::errors::ApiErrorResponse> {
    logger::info!("perform_billing_processor_record_back");

    let subscription = state
        .store
        .find_by_merchant_id_subscription_id(
            merchant_account.get_id(),
            tracking_data.subscription_id.get_string_repr().to_string(),
        )
        .await
        .change_context(crate::errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch subscription from DB")?;

    let customer = state
        .store
        .find_customer_by_customer_id_merchant_id(
            &(state).into(),
            &tracking_data.customer_id,
            merchant_account.get_id(),
            key_store,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(crate::errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("subscriptions: unable to fetch customer from database")?;

    let profile = state
        .store
        .find_business_profile_by_profile_id(&(state).into(), key_store, &tracking_data.profile_id)
        .await
        .change_context(crate::errors::ApiErrorResponse::ProfileNotFound {
            id: tracking_data.profile_id.get_string_repr().to_string(),
        })?;

    let billing_handler = subscription::BillingHandler::create(
        state,
        merchant_account,
        key_store,
        subscription.clone(),
        customer,
        profile.clone(),
        None,
        None,
        None,
    )
    .await
    .change_context(crate::errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to create billing handler")?;

    let invoice_handler = subscription::InvoiceHandler::new(subscription.clone(), merchant_account.clone(), profile);

    let invoice = invoice_handler
        .fetch_invoice_by_id(state, &tracking_data.invoice_id)
        .await?;

    // TODO: Handle retries here on failure
    billing_handler
        .record_back_to_billing_processor(state, tracking_data.connector_invoice_id.clone(), tracking_data.amount, tracking_data.currency)
        .await?;

    invoice_handler
        .update_invoice_status(
            state,
            invoice.id.get_string_repr().to_string(),
            common_enums::connector_enums::InvoiceStatus::InvoicePaid,
        )
        .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn create_invoice_sync_job(
    state: &SessionState,
    request: api_models::process_tracker::invoice_sync::InvoiceSyncRequest,
) -> CustomResult<(), crate::errors::ApiErrorResponse> {
    let tracking_data =
        api_models::process_tracker::invoice_sync::InvoiceSyncTrackingData::from(request);

    let process_tracker_entry = diesel_models::ProcessTrackerNew::new(
        common_utils::generate_id(crate::consts::ID_LENGTH, "proc"),
        IVOICE_SYNC_WORKFLOW.to_string(),
        common_enums::ProcessTrackerRunner::InvoiceSyncflow,
        vec![IVOICE_SYNC_WORKFLOW_TAG.to_string()],
        tracking_data,
        Some(0),
        common_utils::date_time::now(),
        common_types::consts::API_VERSION,
    )
    .change_context(crate::errors::ApiErrorResponse::InternalServerError)?;

    state
        .store
        .insert_process(process_tracker_entry)
        .await
        .change_context(crate::errors::ApiErrorResponse::InternalServerError)?;

    Ok(())
}

pub async fn get_subscription_invoice_sync_process_schedule_time(
    db: &dyn StorageInterface,
    connector: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_count: i32,
) -> Result<Option<time::PrimitiveDateTime>, errors::ProcessTrackerError> {
    // Can have config based mapping as well
    let mapping: CustomResult<
        process_data::SubscriptionInvoiceSyncPTMapping,
        crate::errors::StorageError,
    > = db
        .find_config_by_key(&format!("invoice_sync_pt_mapping_{connector}"))
        .await
        .map(|value| value.config)
        .and_then(|config| {
            config
                .parse_struct("SubscriptionInvoiceSyncPTMapping")
                .change_context(crate::errors::StorageError::DeserializationFailed)
        });
    let mapping = match mapping {
        Ok(x) => x,
        Err(error) => {
            logger::info!(?error, "Redis Mapping Error");
            process_data::SubscriptionInvoiceSyncPTMapping::default()
        }
    };

    let time_delta = scheduler_utils::get_subscription_invoice_sync_retry_schedule_time(
        mapping,
        merchant_id,
        retry_count,
    );

    Ok(scheduler_utils::get_time_from_delta(time_delta))
}

pub async fn retry_subscription_invoice_sync_task(
    db: &dyn StorageInterface,
    connector: String,
    merchant_id: common_utils::id_type::MerchantId,
    pt: storage::ProcessTracker,
) -> Result<bool, errors::ProcessTrackerError> {
    let schedule_time = get_subscription_invoice_sync_process_schedule_time(
        db,
        connector.as_str(),
        &merchant_id,
        pt.retry_count + 1,
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
