use api_models::refunds::{RefundRequest, RefundType};
use common_utils::ext_traits::ValueExt;

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

            let db = &*state.store;
            
            let tracking_data: AutoRefundWorkflow = process
                .tracking_data
                .clone()
                .parse_value("AutoRefundWorkflow")?;

            let key_store = state
                .store
                .get_merchant_key_store_by_merchant_id(
                    tracking_data.merchant_id.as_str(),
                    &state.store.get_master_key().to_vec().into(),
                )
                .await?;

            let merchant_account = db
                .find_merchant_account_by_merchant_id(tracking_data.merchant_id.as_str(), &key_store)
                .await?;

            let ref_req = RefundRequest {
                refund_id: None,
                payment_id: tracking_data.payment_id.as_str(),
                merchant_id: tracking_data.merchant_id.as_str(),
                amount: None,
                reason: "",
                refund_type: Some(RefundType::Scheduled),
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
