use common_utils::{
    consts::DEFAULT_LOCALE,
    ext_traits::{OptionExt, ValueExt},
};
use hyperswitch_domain_models::payments::HeaderPayload;
use scheduler::{
    consumer::{self, workflows::ProcessTrackerWorkflow},
    errors,
};

use crate::{
    core::{configs::dimension_state, payouts},
    errors as core_errors,
    routes::SessionState,
    types::{api, domain, storage},
};

pub struct AttachPayoutAccountWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for AttachPayoutAccountWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
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
                &merchant_id,
                &db.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await?;

        let request = api::payouts::PayoutRequest::PayoutRetrieveRequest(tracking_data);

        let platform = domain::Platform::new(
            merchant_account.clone(),
            key_store.clone(),
            merchant_account,
            key_store,
            None,
        );
        let dimensions = dimension_state::Dimensions::new()
            .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
            .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id());
        let mut payout_data = Box::pin(payouts::make_payout_data(
            state,
            &platform,
            None,
            &request,
            DEFAULT_LOCALE,
        ))
        .await?;

        let dimensions = dimensions.with_profile_id(payout_data.profile_id.clone());

        payouts::payouts_core(state, &platform, &mut payout_data, None, None, &dimensions)
            .await?;

        Ok(())
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        error: errors::ProcessTrackerError,
    ) -> core_errors::CustomResult<(), errors::ProcessTrackerError> {
        consumer::consumer_error_handler(state.store.as_scheduler(), process, error).await
    }
}
