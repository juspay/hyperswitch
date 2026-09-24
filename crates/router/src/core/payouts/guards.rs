use std::str::FromStr;

use common_utils::payout_method_utils;
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};

use super::PayoutData;
use crate::{
    core::{
        blocklist::utils as blocklist_utils,
        configs::dimension_state,
        errors::{self, RouterResult},
    },
    routes::SessionState,
    types::{api::payouts, domain, storage},
};

const BLOCKLIST_ERROR_CODE: &str = "HE-03";

#[derive(Debug, Clone)]
pub enum PayoutGuardOutcome {
    Allow,
    Block(PayoutBlockDetails),
}

#[derive(Debug, Clone)]
pub struct PayoutBlockDetails {
    pub error_code: String,
    pub error_message: Option<String>,
    pub active_frm_id: Option<String>,
}

#[async_trait::async_trait]
trait PayoutGuard {
    async fn should_run(
        &self,
        state: &SessionState,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
    ) -> bool;

    async fn execute(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
        payout_data: &PayoutData,
    ) -> RouterResult<PayoutGuardOutcome>;
}

struct BlocklistGuard;

#[async_trait::async_trait]
impl PayoutGuard for BlocklistGuard {
    async fn should_run(
        &self,
        state: &SessionState,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
    ) -> bool {
        dimensions
            .get_payout_blocklist_guard(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                None,
            )
            .await
    }

    async fn execute(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
        payout_data: &PayoutData,
    ) -> RouterResult<PayoutGuardOutcome> {
        let block_reason = match get_blocklist_payment_method_data(payout_data) {
            Some(payment_method_data) => {
                blocklist_utils::check_blocklist(
                    state,
                    platform.get_processor(),
                    &Some(payment_method_data),
                    &payout_data.business_profile,
                )
                .await?
            }
            None => None,
        };

        Ok(block_reason.map_or(PayoutGuardOutcome::Allow, |reason| {
            logger::warn!(block_reason = ?reason, "Payout blocked by blocklist");
            PayoutGuardOutcome::Block(PayoutBlockDetails {
                error_code: BLOCKLIST_ERROR_CODE.to_string(),
                error_message: Some(reason.error_message()),
                active_frm_id: None,
            })
        }))
    }
}

// A stored connector transfer method skips resolving the PAN, so fall back to the persisted BIN
fn get_blocklist_payment_method_data(
    payout_data: &PayoutData,
) -> Option<domain::EligibilityPaymentMethodData> {
    let card_from_payout_method_data =
        payout_data
            .payout_method_data
            .as_ref()
            .and_then(|payout_method_data| match payout_method_data {
                payouts::PayoutMethodData::Card(card) => Some(
                    domain::EligibilityPaymentMethodData::Card(domain::EligibilityCard::from(card)),
                ),
                _ => None,
            });

    card_from_payout_method_data.or_else(|| {
        payout_data
            .payout_attempt
            .additional_payout_method_data
            .as_ref()
            .and_then(|additional_data| match additional_data {
                payout_method_utils::AdditionalPayoutMethodData::Card(card) => card
                    .card_extended_bin
                    .as_deref()
                    .or(card.card_isin.as_deref())
                    .and_then(|bin| cards::CardBin::from_str(bin).ok()),
                _ => None,
            })
            .map(|card_bin| {
                domain::EligibilityPaymentMethodData::CardBin(domain::EligibilityCardBin {
                    card_bin,
                })
            })
    })
}

async fn run_guard<G: PayoutGuard + Sync>(
    guard: G,
    state: &SessionState,
    platform: &domain::Platform,
    payout_data: &PayoutData,
    dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
) -> RouterResult<PayoutGuardOutcome> {
    match guard.should_run(state, dimensions).await {
        true => guard.execute(state, platform, payout_data).await,
        false => Ok(PayoutGuardOutcome::Allow),
    }
}

async fn run_payout_guards(
    state: &SessionState,
    platform: &domain::Platform,
    payout_data: &PayoutData,
    dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
) -> RouterResult<PayoutGuardOutcome> {
    run_guard(BlocklistGuard, state, platform, payout_data, dimensions).await
}

/// Runs the pre-connector payout guards and marks the payout failed if one blocks it.
/// Returns whether the payout was blocked. Expects `payout_method_data` to be resolved already.
#[instrument(skip_all)]
pub async fn is_payout_blocked(
    state: &SessionState,
    platform: &domain::Platform,
    payout_data: &mut PayoutData,
    dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantId,
) -> RouterResult<bool> {
    let profile_dimensions =
        dimensions.with_profile_id(payout_data.business_profile.get_id().clone());

    match run_payout_guards(state, platform, payout_data, &profile_dimensions).await? {
        PayoutGuardOutcome::Block(block_details) => {
            update_blocked_payout_tracker(state, platform, payout_data, block_details).await?;
            Ok(true)
        }
        PayoutGuardOutcome::Allow => Ok(false),
    }
}

async fn update_blocked_payout_tracker(
    state: &SessionState,
    platform: &domain::Platform,
    payout_data: &mut PayoutData,
    block_details: PayoutBlockDetails,
) -> RouterResult<()> {
    let db = &*state.store;
    let storage_scheme = platform.get_processor().get_account().storage_scheme;

    payout_data.payout_attempt = db
        .update_payout_attempt(
            &payout_data.payout_attempt,
            storage::PayoutAttemptUpdate::StatusUpdate {
                status: common_enums::PayoutStatus::Failed,
                error_code: Some(block_details.error_code),
                error_message: block_details.error_message,
                active_frm_id: block_details.active_frm_id,
                is_eligible: Some(false),
                unified_code: None,
                unified_message: None,
                connector_eligibility_reference_id: None,
                connector_payout_id: None,
                payout_connector_metadata: None,
            },
            &payout_data.payouts,
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payout attempt after pre-connector guard rejection")?;

    payout_data.payouts = db
        .update_payout(
            &payout_data.payouts,
            storage::PayoutsUpdate::StatusUpdate {
                status: common_enums::PayoutStatus::Failed,
            },
            &payout_data.payout_attempt,
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payout after pre-connector guard rejection")?;

    Ok(())
}
