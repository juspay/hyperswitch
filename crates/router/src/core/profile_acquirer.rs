use api_models::profile_acquirer;
use error_stack::ResultExt;

use crate::{
    core::errors::{self, utils::StorageErrorExt, RouterResponse},
    services::api,
    types::domain,
    SessionState,
};

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn create_profile_acquirer(
    state: SessionState,
    request: profile_acquirer::ProfileAcquirerCreate,
    platform: domain::Platform,
) -> RouterResponse<profile_acquirer::ProfileAcquirerResponse> {
    let db = state.store.as_ref();
    let profile_acquirer_id = common_utils::generate_profile_acquirer_id_of_default_length();
    let merchant_key_store = platform.get_processor().get_key_store();

    let mut business_profile = db
        .find_business_profile_by_profile_id(merchant_key_store, &request.profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: request.profile_id.get_string_repr().to_owned(),
        })?;

    let has_existing_default = business_profile
        .acquirer_config_map
        .as_ref()
        .map(|map| map.default_acquirer_config.is_some())
        .unwrap_or(false);

    let is_default = if !has_existing_default {
        true // Always make the first configuration default
    } else {
        request.is_default.unwrap_or(false)
    };

    let incoming_acquirer_config = common_types::domain::AcquirerConfig {
        acquirer_assigned_merchant_id: request.acquirer_assigned_merchant_id.clone(),
        merchant_name: request.merchant_name.clone(),
        network: request.network.clone(),
        acquirer_bin: request.acquirer_bin.clone(),
        acquirer_ica: request.acquirer_ica.clone(),
        acquirer_fraud_rate: request.acquirer_fraud_rate,
        acquirer_country_code: request.acquirer_country_code.clone(),
    };

    // Initialize the new bucket as a Vec containing its first AcquirerConfig entry.
    let configs_map = business_profile.acquirer_config_map.get_or_insert_with(|| {
        common_types::domain::AcquirerConfigMap {
            default_acquirer_config: None,
            configs: std::collections::HashMap::new(),
        }
    });

    configs_map.configs.insert(
        profile_acquirer_id.clone(),
        vec![incoming_acquirer_config.clone()],
    );

    if is_default {
        configs_map.default_acquirer_config = Some(profile_acquirer_id.clone());
    }

    let profile_update = domain::ProfileUpdate::AcquirerConfigMapUpdate {
        acquirer_config_map: business_profile.acquirer_config_map.clone(),
    };
    let updated_business_profile = db
        .update_profile_by_profile_id(merchant_key_store, business_profile, profile_update)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update business profile with new acquirer config")?;

    // Retrieve the specific entry we just inserted from the updated profile.
    let updated_acquirer_config = updated_business_profile
        .acquirer_config_map
        .as_ref()
        .and_then(|wrapper| wrapper.configs.get(&profile_acquirer_id))
        .and_then(|bucket| {
            bucket
                .iter()
                .find(|cfg| cfg.network == incoming_acquirer_config.network)
        })
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get updated acquirer config")?;

    let response = profile_acquirer::ProfileAcquirerResponse::from((
        profile_acquirer_id,
        &request.profile_id,
        Some(updated_acquirer_config),
        is_default,
    ));

    Ok(api::ApplicationResponse::Json(response))
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn update_profile_acquirer_config(
    state: SessionState,
    profile_id: common_utils::id_type::ProfileId,
    profile_acquirer_id: common_utils::id_type::ProfileAcquirerId,
    request: profile_acquirer::ProfileAcquirerUpdate,
    platform: domain::Platform,
) -> RouterResponse<profile_acquirer::ProfileAcquirerResponse> {
    let db = state.store.as_ref();
    let merchant_key_store = platform.get_processor().get_key_store();

    let mut business_profile = db
        .find_business_profile_by_profile_id(merchant_key_store, &profile_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

    let acquirer_config_map = business_profile
        .acquirer_config_map
        .as_mut()
        .ok_or(errors::ApiErrorResponse::ProfileAcquirerNotFound {
            profile_id: profile_id.get_string_repr().to_owned(),
            profile_acquirer_id: profile_acquirer_id.get_string_repr().to_owned(),
        })
        .attach_printable("no acquirer config found in business profile")?;

    let default_bucket_id = acquirer_config_map.default_acquirer_config.clone();

    // Verify the target bucket (profile_acquirer_id) exists.
    let bucket = acquirer_config_map
        .configs
        .get_mut(&profile_acquirer_id)
        .ok_or_else(|| errors::ApiErrorResponse::ProfileAcquirerNotFound {
            profile_id: profile_id.get_string_repr().to_owned(),
            profile_acquirer_id: profile_acquirer_id.get_string_repr().to_owned(),
        })?;

    // `network` is mandatory on update — find whether a slot for this network already exists in the bucket.
    let is_default = request
        .is_default
        .unwrap_or_else(|| default_bucket_id.as_ref() == Some(&profile_acquirer_id));

    // Update default bucket
    let mut default_bucket_changed = false;
    if request.is_default == Some(true) {
        if default_bucket_id.as_ref() != Some(&profile_acquirer_id) {
            acquirer_config_map.default_acquirer_config = Some(profile_acquirer_id.clone());
            default_bucket_changed = true;
        }
    } else if request.is_default == Some(false)
        && default_bucket_id.as_ref() == Some(&profile_acquirer_id)
    {
        acquirer_config_map.default_acquirer_config = None;
        default_bucket_changed = true;
    }

    if let Some(target_network) = request.network.clone() {
        let existing_pos = bucket.iter().position(|cfg| cfg.network == target_network);

        // Build the upserted config by applying partial-update fields over the existing slot
        let upserted_config = {
            let base = existing_pos
                .and_then(|pos| bucket.get(pos).cloned())
                .unwrap_or_else(|| common_types::domain::AcquirerConfig {
                    network: target_network.clone(),
                    acquirer_assigned_merchant_id: String::new(),
                    merchant_name: String::new(),
                    acquirer_bin: String::new(),
                    acquirer_ica: None,
                    acquirer_fraud_rate: None,
                    acquirer_country_code: None,
                });

            common_types::domain::AcquirerConfig {
                acquirer_assigned_merchant_id: request
                    .acquirer_assigned_merchant_id
                    .unwrap_or(base.acquirer_assigned_merchant_id),
                merchant_name: request.merchant_name.unwrap_or(base.merchant_name),
                acquirer_bin: request.acquirer_bin.unwrap_or(base.acquirer_bin),
                acquirer_ica: request.acquirer_ica.or(base.acquirer_ica),
                acquirer_fraud_rate: request.acquirer_fraud_rate.or(base.acquirer_fraud_rate),
                acquirer_country_code: request.acquirer_country_code.or(base.acquirer_country_code),
                network: base.network,
            }
        };

        // intra-bucket duplicate check: if the entry already exists and has the same content,
        // reject it as a duplicate (unless we actually changed the default bucket flag)
        if let Some(pos) = existing_pos {
            if bucket.get(pos) == Some(&upserted_config) && !default_bucket_changed {
                return Err(error_stack::report!(
                    errors::ApiErrorResponse::GenericDuplicateError {
                        message: format!(
                            "An identical configuration for network '{}' already exists in bucket '{}'.",
                            upserted_config.network,
                            profile_acquirer_id.get_string_repr()
                        ),
                    }
                ));
            }
        }

        // Upsert into the bucket: overwrite existing network slot or push a new entry.
        let bucket_mut = acquirer_config_map
            .configs
            .get_mut(&profile_acquirer_id)
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("bucket was verified above but vanished")?;

        match existing_pos {
            Some(pos) => {
                if let Some(slot) = bucket_mut.get_mut(pos) {
                    *slot = upserted_config.clone();
                }
            }
            None => bucket_mut.push(upserted_config.clone()),
        }
    } else if !default_bucket_changed {
        return Err(error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "A `network` must be provided to update an acquirer configuration, unless you are only changing the `is_default` fallback status of this entire bucket.".to_string(),
        }));
    }

    let updated_map_for_db_update = business_profile.acquirer_config_map.clone();

    let profile_update = domain::ProfileUpdate::AcquirerConfigMapUpdate {
        acquirer_config_map: updated_map_for_db_update,
    };

    let updated_business_profile = db
        .update_profile_by_profile_id(merchant_key_store, business_profile, profile_update)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update business profile with updated acquirer config")?;

    let final_acquirer_config = updated_business_profile
        .acquirer_config_map
        .as_ref()
        .and_then(|wrapper| wrapper.configs.get(&profile_acquirer_id))
        .and_then(|bucket| {
            if let Some(nw) = request.network.as_ref() {
                bucket.iter().find(|cfg| cfg.network == *nw)
            } else {
                None // is_default-only update: no specific network was modified
            }
        });

    let response = profile_acquirer::ProfileAcquirerResponse::from((
        profile_acquirer_id,
        &profile_id,
        final_acquirer_config,
        is_default,
    ));

    Ok(api::ApplicationResponse::Json(response))
}
