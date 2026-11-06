use common_utils::id_type;
use error_stack::report;

use crate::{
    core::{
        configs::dimension_state,
        errors::{self, RouterResult},
    },
    routes::SessionState,
    types::domain,
};

/// Reject payout requests when the switch resolves to true for their available OMP dimensions.
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

    if disabled {
        return Err(report!(errors::ApiErrorResponse::PreconditionFailed {
            message: "Payouts are disabled by configuration".to_string(),
        }));
    }

    Ok(())
}

/// A request spanning explicit profiles is rejected if any requested profile is disabled.
/// Merchant-wide requests resolve without a profile dimension.
#[cfg(feature = "olap")]
pub async fn ensure_payouts_enabled_for_profiles(
    state: &SessionState,
    platform: &domain::Platform,
    profile_ids: Option<&[id_type::ProfileId]>,
) -> RouterResult<()> {
    match profile_ids.filter(|ids| !ids.is_empty()) {
        Some(profile_ids) => {
            for profile_id in profile_ids {
                ensure_payouts_enabled(state, platform, Some(profile_id)).await?;
            }
            Ok(())
        }
        None => ensure_payouts_enabled(state, platform, None).await,
    }
}
