use async_trait::async_trait;
use common_enums::connector_enums::InvoiceStatus;
use common_utils::{errors::CustomResult, ext_traits::ValueExt};
use router_env::logger;
use scheduler::{
    consumer::{self, workflows::ProcessTrackerWorkflow},
    errors,
};

use crate::{routes::SessionState, types::storage, utils};

const INVOICE_SYNC_WORKFLOW: &str = "INVOICE_SYNC";

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
            .parse_value::<subscriptions::storage::invoice_sync::InvoiceSyncTrackingData>(
            "InvoiceSyncTrackingData",
        )?;
        let subscription_state = state.clone().into();
        match process.name.as_deref() {
            Some(INVOICE_SYNC_WORKFLOW) => {
                let (handler, payments_response) =
                    Box::pin(subscriptions::workflows::perform_subscription_invoice_sync(
                        &subscription_state,
                        process,
                        tracking_data,
                    ))
                    .await?;

                if handler.invoice.status == InvoiceStatus::InvoicePaid
                    || handler.invoice.status == InvoiceStatus::PaymentSucceeded
                    || handler.invoice.status == InvoiceStatus::PaymentFailed
                {
                    let _ = utils::trigger_subscriptions_outgoing_webhook(
                        state,
                        payments_response,
                        &handler.invoice,
                        &handler.subscription,
                        &handler.merchant_account,
                        &handler.key_store,
                        &handler.profile,
                    )
                    .await
                    .map_err(|e| {
                        logger::error!("Failed to trigger subscriptions outgoing webhook: {e:?}");
                        errors::ProcessTrackerError::FlowExecutionError {
                            flow: "Trigger Subscriptions Outgoing Webhook",
                        }
                    })?;
                }

                Ok(())
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
        Ok(())
    }
}
