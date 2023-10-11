use common_utils::ext_traits::{OptionExt, ValueExt};
use diesel_models::enums;
use scheduler::{
    consumer::{self, workflows::ProcessTrackerWorkflow},
    errors,
};

use crate::{
    core::payouts as payouts_core,
    errors as core_errors,
    routes::AppState,
    types::{api, storage},
};

pub struct StripeAttachExternalAccountWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<AppState> for StripeAttachExternalAccountWorkflow {
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
            payouts_core::make_payout_data(state, &merchant_account, &key_store, &request).await?;
        let (payout_attempt, payouts) = (
            payout_data.payout_attempt.clone(),
            payout_data.payouts.clone(),
        );

        // Fetch payout method data
        payout_data.payout_method_data = Some(
            payouts_core::helpers::make_payout_method_data(
                state,
                None,
                payout_attempt.payout_token.as_deref(),
                &payout_attempt.customer_id,
                &payout_attempt.merchant_id,
                &payout_attempt.payout_id,
                Some(&payouts.payout_type),
            )
            .await?
            .get_required_value("payout_method_data")?,
        );

        // Form connector data
        let routed_through = Some(payout_attempt.connector.to_owned());
        let connector_data =
            payouts_core::get_connector_data(state, &merchant_account, routed_through, None)
                .await?;

        // 1. Attach recipient's external accounts
        payout_data = payouts_core::create_recipient_account(
            state,
            &merchant_account,
            &key_store,
            &connector_data,
            &mut payout_data,
        )
        .await?;

        if payout_attempt.confirm
            && payout_data.payout_attempt.status == enums::PayoutStatus::RequiresCreation
        {
            // 2. Create payout
            payout_data = payouts_core::create_payout(
                state,
                &merchant_account,
                &key_store,
                &connector_data,
                &mut payout_data,
            )
            .await?;
        }

        if payouts.auto_fulfill
            && payout_data.payout_attempt.status == enums::PayoutStatus::RequiresFulfillment
        {
            // 3. Fulfill payout
            payouts_core::fulfill_payout(
                state,
                &merchant_account,
                &key_store,
                &connector_data,
                &mut payout_data,
            )
            .await?;
        }

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
