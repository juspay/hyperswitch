use common_utils::ext_traits::ValueExt;
use diesel_models::enums::{self as storage_enums};

use super::{ProcessTrackerWorkflow,AutoRefundWorkflow};
use crate::{
    errors, logger::error, routes::AppState, types::storage, core::refunds::refund_create_core,
};

#[async_trait::async_trait]
impl ProcessTrackerWorkflow for AutoRefundWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
            let ref_req = RefundRequest {
                refund_id: None,
                payment_id: payment_intent.payment_id.clone(),
                merchant_id: Some(merchant_account.merchant_id.clone()),
                amount: None,
                reason: "",
                refund_type: Some(refunds::RefundType::Instant),
                metadata: None,
                merchant_connector_details: None,
            };
        Ok(refund_create_core(state, merchant_account.clone(), key_store, ref_req,).await?)
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
