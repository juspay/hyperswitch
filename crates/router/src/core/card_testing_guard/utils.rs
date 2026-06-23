use error_stack::ResultExt;
use hyperswitch_domain_models::{
    card_testing_guard_data::CardTestingGuardData, router_request_types::BrowserInformation,
};
use hyperswitch_masking::{PeekInterface, Secret};
use router_env::logger;

use super::errors;
use crate::{
    core::{errors::RouterResult, payments::helpers},
    routes::SessionState,
    services,
    types::domain,
    utils::crypto::{self, SignMessage},
};

pub async fn validate_card_testing_guard_checks(
    state: &SessionState,
    #[cfg(feature = "v1")] browser_info: Option<&serde_json::Value>,
    #[cfg(feature = "v2")] browser_info: Option<&BrowserInformation>,
    card_number: cards::CardNumber,
    customer_id: &Option<common_utils::id_type::CustomerId>,
    business_profile: &domain::Profile,
) -> RouterResult<Option<CardTestingGuardData>> {
    match &business_profile.card_testing_guard_config {
        Some(card_testing_guard_config) => {
            let fingerprint = generate_fingerprint(card_number, business_profile).await?;

            let ip_address = extract_ip_from_browser_info(browser_info);

            let card_ip_blocking_cache_key =
                if card_testing_guard_config.is_card_ip_blocking_enabled {
                    match ip_address {
                        Some(ip) => Some(
                            helpers::validate_card_ip_blocking_for_business_profile(
                                state,
                                ip,
                                fingerprint.clone(),
                                card_testing_guard_config,
                            )
                            .await?,
                        ),
                        None => None,
                    }
                } else {
                    None
                };

            let guest_user_card_blocking_cache_key =
                if card_testing_guard_config.is_guest_user_card_blocking_enabled {
                    Some(
                        helpers::validate_guest_user_card_blocking_for_business_profile(
                            state,
                            fingerprint.clone(),
                            customer_id.clone(),
                            card_testing_guard_config,
                        )
                        .await?,
                    )
                } else {
                    None
                };

            let customer_id_blocking_cache_key =
                if card_testing_guard_config.is_customer_id_blocking_enabled {
                    match customer_id.clone() {
                        Some(cid) => Some(
                            helpers::validate_customer_id_blocking_for_business_profile(
                                state,
                                cid,
                                business_profile.get_id(),
                                card_testing_guard_config,
                            )
                            .await?,
                        ),
                        None => None,
                    }
                } else {
                    None
                };

            let guest_ip_blocking_cache_key = if card_testing_guard_config
                .is_guest_ip_blocking_enabled
                && customer_id.is_none()
            {
                match ip_address {
                    Some(ip) => {
                        match helpers::validate_guest_ip_blocking_for_business_profile(
                            state,
                            ip,
                            business_profile.get_id(),
                            card_testing_guard_config,
                        )
                        .await
                        {
                            Ok(cache_key) => Ok(Some(cache_key)),
                            Err(err) => match err.current_context() {
                                errors::ApiErrorResponse::PreconditionFailed { .. } => Err(err),
                                _ => {
                                    logger::error!("Guest IP blocking validation error: {:?}", err);
                                    Ok(None)
                                }
                            },
                        }
                    }
                    None => Ok(None),
                }
            } else {
                Ok(None)
            }?;

            Ok(Some(CardTestingGuardData {
                is_card_ip_blocking_enabled: card_testing_guard_config.is_card_ip_blocking_enabled,
                card_ip_blocking_cache_key,
                is_guest_user_card_blocking_enabled: card_testing_guard_config
                    .is_guest_user_card_blocking_enabled,
                guest_user_card_blocking_cache_key,
                is_customer_id_blocking_enabled: card_testing_guard_config
                    .is_customer_id_blocking_enabled,
                customer_id_blocking_cache_key,
                card_testing_guard_expiry: card_testing_guard_config.card_testing_guard_expiry,
                is_guest_ip_blocking_enabled: card_testing_guard_config
                    .is_guest_ip_blocking_enabled,
                guest_ip_blocking_cache_key,
            }))
        }
        None => Ok(None),
    }
}

fn extract_ip_from_browser_info(
    #[cfg(feature = "v1")] browser_info: Option<&serde_json::Value>,
    #[cfg(feature = "v2")] browser_info: Option<&BrowserInformation>,
) -> Option<std::net::IpAddr> {
    #[cfg(feature = "v1")]
    {
        browser_info.and_then(|info| {
            match serde_json::from_value::<BrowserInformation>(info.clone()) {
                Ok(b) => b.ip_address,
                Err(err) => {
                    logger::error!(
                        "Failed to deserialize browser_info for IP extraction: {:?}",
                        err
                    );
                    None
                }
            }
        })
    }

    #[cfg(feature = "v2")]
    {
        browser_info.and_then(|b| b.ip_address)
    }
}

pub async fn generate_fingerprint(
    card_number: cards::CardNumber,
    business_profile: &domain::Profile,
) -> RouterResult<Secret<String>> {
    let card_testing_secret_key = &business_profile.card_testing_secret_key;

    match card_testing_secret_key {
        Some(card_testing_secret_key) => {
            let card_number_fingerprint = crypto::HmacSha512::sign_message(
                &crypto::HmacSha512,
                card_testing_secret_key.get_inner().peek().as_bytes(),
                card_number.clone().get_card_no().as_bytes(),
            )
            .attach_printable("error in pm fingerprint creation")
            .map_or_else(
                |err| {
                    logger::error!(error=?err);
                    None
                },
                Some,
            )
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
) {
    if let Some(card_testing_guard_data) = card_testing_guard_data {
        if card_testing_guard_data.is_card_ip_blocking_enabled {
            if let Some(ref cache_key) = card_testing_guard_data.card_ip_blocking_cache_key {
                if let Err(err) = services::card_testing_guard::increment_blocked_count_in_cache(
                    state,
                    cache_key,
                    card_testing_guard_data.card_testing_guard_expiry.into(),
                )
                .await
                {
                    logger::error!(
                        "Failed to increment card IP blocked count in cache: {:?}",
                        err
                    );
                }
            }
        }

        if card_testing_guard_data.is_guest_user_card_blocking_enabled {
            if let Some(ref cache_key) = card_testing_guard_data.guest_user_card_blocking_cache_key
            {
                if let Err(err) = services::card_testing_guard::increment_blocked_count_in_cache(
                    state,
                    cache_key,
                    card_testing_guard_data.card_testing_guard_expiry.into(),
                )
                .await
                {
                    logger::error!(
                        "Failed to increment guest user card blocked count in cache: {:?}",
                        err
                    );
                }
            }
        }

        if card_testing_guard_data.is_customer_id_blocking_enabled {
            if let Some(ref cache_key) = card_testing_guard_data.customer_id_blocking_cache_key {
                if let Err(err) = services::card_testing_guard::increment_blocked_count_in_cache(
                    state,
                    cache_key,
                    card_testing_guard_data.card_testing_guard_expiry.into(),
                )
                .await
                {
                    logger::error!(
                        "Failed to increment customer ID blocked count in cache: {:?}",
                        err
                    );
                }
            }
        }

        if card_testing_guard_data.is_guest_ip_blocking_enabled {
            if let Some(ref cache_key) = card_testing_guard_data.guest_ip_blocking_cache_key {
                if let Err(err) = services::card_testing_guard::increment_blocked_count_in_cache(
                    state,
                    cache_key,
                    card_testing_guard_data.card_testing_guard_expiry.into(),
                )
                .await
                {
                    logger::error!(
                        "Failed to increment guest IP blocked count in cache: {:?}",
                        err
                    );
                }
            }
        }
    }
}
