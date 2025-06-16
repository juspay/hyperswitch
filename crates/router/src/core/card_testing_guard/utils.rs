use error_stack::ResultExt;
use hyperswitch_domain_models::{
    card_testing_guard_data::CardTestingGuardData, router_request_types::BrowserInformation,
};
use masking::{PeekInterface, Secret};
use router_env::logger;

use super::errors;
use crate::{
    core::{errors::RouterResult, payments::helpers},
    routes::SessionState,
    services,
    types::{api, domain},
    utils::crypto::{self, SignMessage},
};

pub async fn validate_card_testing_guard_checks(
    state: &SessionState,
    request: &api::PaymentsRequest,
    payment_method_data: Option<&api_models::payments::PaymentMethodData>,
    customer_id: &Option<common_utils::id_type::CustomerId>,
    business_profile: &domain::Profile,
) -> RouterResult<Option<CardTestingGuardData>> {
    match &business_profile.card_testing_guard_config {
        Some(card_testing_guard_config) => {
            let fingerprint = generate_fingerprint(payment_method_data, business_profile).await?;

            let card_testing_guard_expiry = card_testing_guard_config.card_testing_guard_expiry;

            let mut card_ip_blocking_cache_key = String::new();
            let mut guest_user_card_blocking_cache_key = String::new();
            let mut customer_id_blocking_cache_key = String::new();

            if card_testing_guard_config.is_card_ip_blocking_enabled {
                if let Some(browser_info) = &request.browser_info {
                    #[cfg(feature = "v1")]
                    {
                        let browser_info =
                            serde_json::from_value::<BrowserInformation>(browser_info.clone())
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("could not parse browser_info")?;

                        if let Some(browser_info_ip) = browser_info.ip_address {
                            card_ip_blocking_cache_key =
                                helpers::validate_card_ip_blocking_for_business_profile(
                                    state,
                                    browser_info_ip,
                                    fingerprint.clone(),
                                    card_testing_guard_config,
                                )
                                .await?;
                        }
                    }

                    #[cfg(feature = "v2")]
                    {
                        if let Some(browser_info_ip) = browser_info.ip_address {
                            card_ip_blocking_cache_key =
                                helpers::validate_card_ip_blocking_for_business_profile(
                                    state,
                                    browser_info_ip,
                                    fingerprint.clone(),
                                    card_testing_guard_config,
                                )
                                .await?;
                        }
                    }
                }
            }

            if card_testing_guard_config.is_guest_user_card_blocking_enabled {
                guest_user_card_blocking_cache_key =
                    helpers::validate_guest_user_card_blocking_for_business_profile(
                        state,
                        fingerprint.clone(),
                        customer_id.clone(),
                        card_testing_guard_config,
                    )
                    .await?;
            }

            if card_testing_guard_config.is_customer_id_blocking_enabled {
                if let Some(customer_id) = customer_id.clone() {
                    customer_id_blocking_cache_key =
                        helpers::validate_customer_id_blocking_for_business_profile(
                            state,
                            customer_id.clone(),
                            business_profile.get_id(),
                            card_testing_guard_config,
                        )
                        .await?;
                }
            }

            Ok(Some(CardTestingGuardData {
                is_card_ip_blocking_enabled: card_testing_guard_config.is_card_ip_blocking_enabled,
                card_ip_blocking_cache_key,
                is_guest_user_card_blocking_enabled: card_testing_guard_config
                    .is_guest_user_card_blocking_enabled,
                guest_user_card_blocking_cache_key,
                is_customer_id_blocking_enabled: card_testing_guard_config
                    .is_customer_id_blocking_enabled,
                customer_id_blocking_cache_key,
                card_testing_guard_expiry,
            }))
        }
        None => Ok(None),
    }
}

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
    card_testing_guard_data: Option<CardTestingGuardData>,
) -> RouterResult<()> {
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
    Ok(())
}
