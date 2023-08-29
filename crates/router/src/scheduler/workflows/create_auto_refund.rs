use api_models::{
    enums::EventType,
    refunds::{RefundRequest, RefundType},
    webhooks::OutgoingWebhookContent,
};
use common_utils::ext_traits::ValueExt;
use diesel_models::{
    enums::{EventClass, EventObjectType},
    refund,
};

use super::{AutoRefundWorkflow, ProcessTrackerWorkflow};
use crate::{
    core::{
        errors::ApiErrorResponse, refunds::refund_create_core,
        webhooks::create_event_and_trigger_appropriate_outgoing_webhook,
    },
    errors,
    logger::error,
    routes::AppState,
    types::storage,
};

#[async_trait::async_trait]
impl ProcessTrackerWorkflow for AutoRefundWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        let tracking_data: refund::AutoRefundWorkflow = process
            .tracking_data
            .clone()
            .parse_value("AutoRefundWorkflow")?;
        let payment_intent = tracking_data.payment_intent;
        let retry_count = tracking_data.retry_count;
        let max_retries = tracking_data.max_retries;
        if retry_count > max_retries {
            return Err(errors::ProcessTrackerError::FlowExecutionError {
                flow: "RefundCreate",
            });
        }
        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                payment_intent.merchant_id.as_str(),
                &state.store.get_master_key().to_vec().into(),
            )
            .await?;
        let merchant_account = db
            .find_merchant_account_by_merchant_id(payment_intent.merchant_id.as_str(), &key_store)
            .await?;
        let ref_req = RefundRequest {
            refund_id: None,
            payment_id: payment_intent.payment_id,
            merchant_id: Some(payment_intent.merchant_id),
            amount: None,
            reason: Some("Auto Refund".to_string()),
            refund_type: Some(RefundType::Scheduled),
            metadata: None,
            merchant_connector_details: None,
        };
        let refund_flow_result =
            refund_create_core(state, merchant_account.clone(), key_store, ref_req).await;
        match refund_flow_result {
            Ok(refund_response) => {
                match refund_response {
                    crate::services::ApplicationResponse::Json(refund_details) => {
                        create_event_and_trigger_appropriate_outgoing_webhook(
                            state.clone(),
                            merchant_account,
                            EventType::RefundSucceeded,
                            EventClass::Refunds,
                            None,
                            refund_details.clone().refund_id,
                            EventObjectType::RefundDetails,
                            OutgoingWebhookContent::RefundDetails(refund_details),
                        )
                        .await?;
                    }
                    _ => {
                        return Err(errors::ProcessTrackerError::UnexpectedFlow);
                    }
                };
            }
            Err(err) => {
                let error = err.current_context().clone();
                match error {
                    ApiErrorResponse::InvalidJwtToken
                    | ApiErrorResponse::ExternalConnectorError { .. }
                    | ApiErrorResponse::RefundFailed { .. } => {
                        // retry refund
                    }
                    _ => {
                        return Err(errors::ProcessTrackerError::FlowExecutionError {
                            flow: "RefundCreate",
                        });
                    }
                }
            }
        };
        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        _state: &'a AppState,
        process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        error!(%process.id, "Failed while executing workflow");
        Ok(())
    }
}
