use cards::CardNumber;
use error_stack::ResultExt;
use hyperswitch_masking::StrongSecret;

use crate::{
    core::{
        blocklist::{transformers::generate_fingerprint, utils::get_merchant_fingerprint_secret},
        errors::{self, RouterResult},
    },
    routes::SessionState,
    types::domain,
};

/// PAN-free per-merchant card fingerprint sent to Offer Engine as `card_alias`,
/// the once-per-card velocity key. `Ok(None)` when no card is present; `Err` when
/// a card is present but its key can't be generated, so callers can fail closed
/// rather than let a card bypass velocity.
pub async fn resolve_card_alias(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    card_number: Option<&CardNumber>,
) -> RouterResult<Option<String>> {
    let Some(card_number) = card_number else {
        return Ok(None);
    };
    let secret = get_merchant_fingerprint_secret(state, merchant_account).await?;
    generate_fingerprint(
        state,
        StrongSecret::new(card_number.get_card_no()),
        StrongSecret::new(secret),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("offer velocity: card fingerprint generation failed")
    .map(|payload| Some(payload.fingerprint_id))
}
