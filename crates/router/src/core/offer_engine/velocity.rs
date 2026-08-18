use cards::CardNumber;
use hyperswitch_masking::StrongSecret;

use crate::{
    core::blocklist::{transformers::generate_fingerprint, utils::get_merchant_fingerprint_secret},
    logger,
    routes::SessionState,
    types::domain,
};

/// PAN-free per-merchant card fingerprint sent to Offer Engine as `card_alias`,
/// the once-per-card velocity key. `None` if it can't be generated; callers fail
/// closed rather than let a card bypass velocity.
pub async fn generate_card_alias(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    card_number: &CardNumber,
) -> Option<String> {
    let secret = get_merchant_fingerprint_secret(state, merchant_account)
        .await
        .map_err(|error| logger::warn!(?error, "offer velocity: fingerprint secret unavailable"))
        .ok()?;
    generate_fingerprint(
        state,
        StrongSecret::new(card_number.get_card_no()),
        StrongSecret::new(secret),
    )
    .await
    .map_err(|error| logger::warn!(?error, "offer velocity: card fingerprint generation failed"))
    .ok()
    .map(|payload| payload.fingerprint_id)
}
