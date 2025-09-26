use async_trait::async_trait;
use common_enums::connector_enums::InvoiceStatus;
use common_utils::ext_traits::ValueExt;
use diesel_models::{invoice::InvoiceNew, process_tracker::business_status};
use error_stack::ResultExt;
use scheduler::{consumer::workflows::ProcessTrackerWorkflow, errors};

use crate::{
    core::errors::RecoveryError::ProcessTrackerFailure,
    routes::SessionState,
    types::{domain, storage},
};
pub struct ExecuteSubscriptionWorkflow;

#[async_trait]
impl ProcessTrackerWorkflow<SessionState> for ExecuteSubscriptionWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data = process
            .tracking_data
            .clone()
            .parse_value::<api_models::process_tracker::subscription::SubscriptionWorkflowTrackingData>(
                "SubscriptionWorkflowTrackingData",
            )?;

        match process.name.as_deref() {
            Some("SUBSCRIPTION_MIT_PAYMENT") => {
                Box::pin(perform_subscription_mit_payment(
                    state,
                    &process,
                    &tracking_data,
                ))
                .await
            }
            _ => Err(errors::ProcessTrackerError::JobNotFound),
        }
    }
}

async fn perform_subscription_mit_payment(
    state: &SessionState,
    process: &storage::ProcessTracker,
    tracking_data: &api_models::process_tracker::subscription::SubscriptionWorkflowTrackingData,
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

    let invoice_new = InvoiceNew {
        id: tracking_data.invoice_id.clone(),
        subscription_id: tracking_data.subscription_id.clone(),
        merchant_id: tracking_data.merchant_id.clone(),
        profile_id: tracking_data.profile_id.clone(),
        merchant_connector_id: tracking_data.billing_connector_mca_id.clone(),
        payment_intent_id: None,
        payment_method_id: tracking_data.payment_method_id.clone(),
        customer_id: tracking_data.customer_id.clone(),
        amount: tracking_data.amount,
        currency: tracking_data.currency.to_string(),
        status: InvoiceStatus::PaymentPending.to_string(),
        provider_name: tracking_data.connector_name,
        metadata: None,
        created_at: common_utils::date_time::now(),
        modified_at: common_utils::date_time::now(),
    };

    let _invoice = state.store.insert_invoice_entry(invoice_new).await?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(
            key_manager_state,
            &tracking_data.merchant_id,
            &key_store,
        )
        .await?;

    let profile = state
        .store
        .find_business_profile_by_profile_id(
            key_manager_state,
            &key_store,
            &tracking_data.profile_id,
        )
        .await?;

    let _merchant_context = domain::MerchantContext::NormalMerchant(Box::new(domain::Context(
        merchant_account,
        key_store,
    )));

    let _profile_id = profile.get_id().clone();

    //make a s2s call to payments

    //based on the payment status trigger invoice sync

    //update invoice table

    //and mark the process tracker as complete

    let updated_process = storage::ProcessTracker {
        id: process.id.clone(),
        status: common_enums::ProcessTrackerStatus::Finish,
        ..process.clone()
    };

    state
        .store
        .as_scheduler()
        .finish_process_with_business_status(
            updated_process.clone(),
            business_status::EXECUTE_WORKFLOW_COMPLETE,
        )
        .await
        .change_context(ProcessTrackerFailure)
        .attach_printable("Failed to update the process tracker")?;

    Ok(())
}
