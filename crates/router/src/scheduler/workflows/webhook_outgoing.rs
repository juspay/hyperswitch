use common_utils::ext_traits::{AsyncExt, ValueExt};

use super::{OutgoingWebhookWorkflow, ProcessTrackerWorkflow};
use crate::{
    core::{
        errors,
        webhooks::{self, types::OutgoingWebhookTrigger},
    },
    db::StorageInterface,
    routes::AppState,
    scheduler::{consumer, utils as pt_utils},
    types::{domain, storage},
};

#[async_trait::async_trait]
impl ProcessTrackerWorkflow for OutgoingWebhookWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data: WebhookWorkflowData = process
            .tracking_data
            .clone()
            .parse_value("WebhookWorkflowData")?;
        Ok(tracking_data
            .trigger_outgoing_webhook::<api_models::webhooks::OutgoingWebhook>(state)
            .await?)
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state, process, error).await
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WebhookWorkflowData {
    PaymentWebhook(diesel_models::payment_intent::PaymentIntent),
    RefundWebhook(diesel_models::refund::Refund),
    DisputeWebhook(diesel_models::dispute::Dispute),
}

impl From<diesel_models::payment_intent::PaymentIntent> for WebhookWorkflowData {
    fn from(value: diesel_models::payment_intent::PaymentIntent) -> Self {
        Self::PaymentWebhook(value)
    }
}

impl From<diesel_models::refund::Refund> for WebhookWorkflowData {
    fn from(value: diesel_models::refund::Refund) -> Self {
        Self::RefundWebhook(value)
    }
}

impl From<diesel_models::dispute::Dispute> for WebhookWorkflowData {
    fn from(value: diesel_models::dispute::Dispute) -> Self {
        Self::DisputeWebhook(value)
    }
}

#[async_trait::async_trait]
impl OutgoingWebhookTrigger for WebhookWorkflowData {
    async fn construct_outgoing_webhook_content(
        &self,
        state: &AppState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: domain::MerchantKeyStore,
    ) -> errors::CustomResult<api_models::webhooks::OutgoingWebhookContent, errors::ApiErrorResponse>
    {
        match self {
            Self::PaymentWebhook(payment_intent) => {
                payment_intent
                    .construct_outgoing_webhook_content(state, merchant_account, merchant_key_store)
                    .await
            }
            Self::RefundWebhook(refund) => {
                refund
                    .construct_outgoing_webhook_content(state, merchant_account, merchant_key_store)
                    .await
            }
            Self::DisputeWebhook(dispute) => {
                dispute
                    .construct_outgoing_webhook_content(state, merchant_account, merchant_key_store)
                    .await
            }
        }
    }

    async fn trigger_outgoing_webhook<W: webhooks::types::OutgoingWebhookType>(
        &self,
        state: &AppState,
    ) -> errors::CustomResult<(), errors::ApiErrorResponse> {
        match self {
            Self::PaymentWebhook(payment_intent) => {
                payment_intent.trigger_outgoing_webhook::<W>(state).await
            }
            Self::RefundWebhook(refund) => refund.trigger_outgoing_webhook::<W>(state).await,
            Self::DisputeWebhook(dispute) => dispute.trigger_outgoing_webhook::<W>(state).await,
        }
    }
}

// This function will take the struct and return it back as it is, while trggering the webhook if
// it fails to add it in the database, it will log the error, and return the dame struct back

pub async fn schedule_outgoing_workflow_event<T: Into<WebhookWorkflowData> + Clone>(
    db: &dyn StorageInterface,
    workflow_data: T,
) -> T {
    let runner = "OUTGOING_WEBHOOK_WORKFLOW";
    let task = "OUTGOING_WEBHOOK";

    // schedule time in seconds (represents the delta from the current time)
    let schedule_time = 30;

    let process_tracker_id =
        common_utils::generate_id_with_default_len(&format!("{}_{}", runner, task));

    let process_tracker_entry =
        <storage::ProcessTracker as storage::ProcessTrackerExt>::make_process_tracker_new(
            process_tracker_id,
            task,
            runner,
            workflow_data.clone().into(),
            pt_utils::get_time_from_delta(Some(schedule_time))
                .unwrap_or(common_utils::date_time::now()),
        );

    match process_tracker_entry
        .async_and_then(|new_entry| async { Ok(db.insert_process(new_entry).await?) })
        .await
    {
        Ok(_) => {}
        Err(_) => {}
    }
    workflow_data
}
