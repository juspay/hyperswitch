use common_utils::ext_traits::{OptionExt, ValueExt};
use scheduler::{
    consumer::{self, workflows::ProcessTrackerWorkflow},
    errors,
};

use crate::{
    core::payouts,
    errors as core_errors,
    routes::AppState,
    types::{api, storage},
};

pub struct StripeAttachAccountWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for StripeAttachAccountWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        // Gather context
        let db = &*state.store;
        let tracking_data: api::PayoutRetrieveRequest = process
            .tracking_data
            .clone()
            .parse_value("PayoutRetrieveRequest")?;

        let merchant_id = tracking_data
            .merchant_id
            .clone()
            .get_required_value("merchant_id")?;

        let key_store = db
            .get_merchant_key_store_by_merchant_id(
                merchant_id.as_ref(),
                &db.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await?;

        let request = api::payouts::PayoutRequest::PayoutRetrieveRequest(tracking_data);

        let mut payout_data =
            payouts::make_payout_data(state, &merchant_account, &key_store, &request).await?;

        payouts::payouts_core(
            state,
            &merchant_account,
            &key_store,
            &mut payout_data,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a AppState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> core_errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}
