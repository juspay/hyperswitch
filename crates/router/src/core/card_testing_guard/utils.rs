use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use router_env::logger;

use super::errors;
use crate::{
    core::errors::RouterResult,
    routes::SessionState,
    services,
    types::domain,
    utils::crypto::{self, SignMessage},
};

pub async fn generate_fingerprint(
    payment_method_data: Option<&api_models::payments::PaymentMethodData>,
    business_profile: &domain::Profile,
) -> RouterResult<Secret<String>> {
    let card_testing_secret_key = &business_profile.card_testing_secret_key;

    match card_testing_secret_key {
        Some(card_testing_secret_key) => {
            let card_number_fingerprint = payment_method_data
                .as_ref()
                .and_then(|pm_data| match pm_data {
                    api_models::payments::PaymentMethodData::Card(card) => {
                        crypto::HmacSha512::sign_message(
                            &crypto::HmacSha512,
                            card_testing_secret_key.get_inner().peek().as_bytes(),
                            card.card_number.clone().get_card_no().as_bytes(),
                        )
                        .attach_printable("error in pm fingerprint creation")
                        .map_or_else(
                            |err| {
                                logger::error!(error=?err);
                                None
                            },
                            Some,
                        )
                    }
                    _ => None,
                })
                .map(hex::encode);

            card_number_fingerprint.map(Secret::new).ok_or_else(|| {
                error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while masking fingerprint")
            })
        }
        None => Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("card testing secret key not configured")?,
    }
}

pub async fn increment_blocked_count_in_cache(
    state: &SessionState,
    card_testing_guard_data: Option<
        hyperswitch_domain_models::card_testing_guard_data::CardTestingGuardData,
    >,
) {
    if let Some(card_testing_guard_data) = card_testing_guard_data.clone() {
        if card_testing_guard_data.is_card_ip_blocking_enabled
            && !card_testing_guard_data
                .card_ip_blocking_cache_key
                .is_empty()
        {
            let _ = services::card_testing_guard::increment_blocked_count_in_cache(
                state,
                &card_testing_guard_data.card_ip_blocking_cache_key,
                card_testing_guard_data.card_testing_guard_expiry.into(),
            )
            .await;
        }

        if card_testing_guard_data.is_guest_user_card_blocking_enabled
            && !card_testing_guard_data
                .guest_user_card_blocking_cache_key
                .is_empty()
        {
            let _ = services::card_testing_guard::increment_blocked_count_in_cache(
                state,
                &card_testing_guard_data.guest_user_card_blocking_cache_key,
                card_testing_guard_data.card_testing_guard_expiry.into(),
            )
            .await;
        }

        if card_testing_guard_data.is_customer_id_blocking_enabled
            && !card_testing_guard_data
                .customer_id_blocking_cache_key
                .is_empty()
        {
            let _ = services::card_testing_guard::increment_blocked_count_in_cache(
                state,
                &card_testing_guard_data.customer_id_blocking_cache_key,
                card_testing_guard_data.card_testing_guard_expiry.into(),
            )
            .await;
        }
    }
}
