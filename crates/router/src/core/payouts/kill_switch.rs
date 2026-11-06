use common_utils::id_type;
use error_stack::report;

pub use crate::core::configs::dimension_config::PayoutKillSwitch;
use crate::{
    core::{
        configs::dimension_state,
        errors::{self, RouterResult},
    },
    routes::SessionState,
    types::domain,
};

impl PayoutKillSwitch {
    /// Reject payout mutations when the switch resolves to true for their available OMP dimensions.
    pub async fn ensure_payouts_enabled(
        state: &SessionState,
        platform: &domain::Platform,
        profile_id: Option<&id_type::ProfileId>,
    ) -> RouterResult<()> {
        let dimensions = dimension_state::Dimensions::new()
            .with_organization_id(platform.get_processor().get_account().get_org_id().clone())
            .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
            .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id());

        let disabled = match profile_id {
            Some(profile_id) => {
                dimensions
                    .with_profile_id(profile_id.clone())
                    .get_payout_kill_switch(
                        state.store.as_ref(),
                        state.superposition_service.as_ref(),
                        None,
                    )
                    .await
            }
            None => {
                dimensions
                    .get_payout_kill_switch(
                        state.store.as_ref(),
                        state.superposition_service.as_ref(),
                        None,
                    )
                    .await
            }
        };

        match disabled {
            true => Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "Payouts are disabled by configuration".to_string(),
            })),
            false => Ok(()),
        }
    }
}
