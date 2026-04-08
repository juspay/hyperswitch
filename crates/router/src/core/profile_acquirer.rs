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
        .map(|map| {
            map.0
                .values()
                .any(|bucket| bucket.iter().any(|config| config.is_default))
        })
        .unwrap_or(false);

    let is_default = match request.is_default {
        Some(true) if has_existing_default => {
            Err(error_stack::report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "A default acquirer configuration already exists for this profile. Please unset the current default before setting a new one.".to_string(),
            }))
        }
        _ => Ok(false)
    }?;

    let incoming_acquirer_config = common_types::domain::AcquirerConfig {
        acquirer_assigned_merchant_id: request.acquirer_assigned_merchant_id.clone(),
        merchant_name: request.merchant_name.clone(),
        network: request.network.clone(),
        acquirer_bin: request.acquirer_bin.clone(),
        acquirer_ica: request.acquirer_ica.clone(),
        acquirer_fraud_rate: request.acquirer_fraud_rate,
        acquirer_country_code: request.acquirer_country_code.clone(),
        is_default,
    };

    // Initialize the new bucket as a Vec containing its first AcquirerConfig entry.
    let configs_map = &mut business_profile
        .acquirer_config_map
        .get_or_insert_with(|| {
            common_types::domain::AcquirerConfigMap(std::collections::HashMap::new())
        })
        .0;

    configs_map.insert(
        profile_acquirer_id.clone(),
        vec![incoming_acquirer_config.clone()],
    );

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
        .and_then(|wrapper| wrapper.0.get(&profile_acquirer_id))
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
        updated_acquirer_config,
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

    // Check if another bucket is already set as default BEFORE borrowing the bucket mutably.
    let has_other_default = acquirer_config_map
        .0
        .iter()
        .any(|(bucket_id, bucket_list)| {
            bucket_id != &profile_acquirer_id && bucket_list.iter().any(|config| config.is_default)
        });

    // Verify the target bucket (profile_acquirer_id) exists.
    let bucket = acquirer_config_map
        .0
        .get_mut(&profile_acquirer_id)
        .ok_or_else(|| errors::ApiErrorResponse::ProfileAcquirerNotFound {
            profile_id: profile_id.get_string_repr().to_owned(),
            profile_acquirer_id: profile_acquirer_id.get_string_repr().to_owned(),
        })?;

    // `network` is mandatory on update — find whether a slot for this network already exists in the bucket.
    let target_network = request.network.clone();
    let existing_pos = bucket.iter().position(|cfg| cfg.network == target_network);

    // Build the upserted config by applying partial-update fields over the existing slot
    // (or blank defaults for a new slot) — all in one immutable expression.
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
                is_default: false,
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
            is_default: request.is_default.unwrap_or(base.is_default),
        }
    };

    // If the updated config is set as default, ensure no OTHER bucket is already default.
    if upserted_config.is_default {
        if has_other_default && request.is_default == Some(true) {
            return Err(error_stack::report!(errors::ApiErrorResponse::PreconditionFailed {
                message: "A default acquirer configuration already exists for this profile. Please unset it before marking this bucket as default.".to_string(),
            }));
        }

        // Sync the is_default flag across all entries in the SAME bucket
        for config in bucket.iter_mut() {
            config.is_default = true;
        }
    } else {
        // If this specific config was explicitly set to NOT default,
        // sync this across the entire bucket.
        if request.is_default == Some(false) {
            for config in bucket.iter_mut() {
                config.is_default = false;
            }
        }
    }

    // intra-bucket duplicate check: if the entry already exists and has the same content,
    // reject it as a duplicate.
    if let Some(pos) = existing_pos {
        if bucket.get(pos) == Some(&upserted_config) {
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
    let bucket = acquirer_config_map
        .0
        .get_mut(&profile_acquirer_id)
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("bucket was verified above but vanished")?;

    match existing_pos {
        Some(pos) => {
            if let Some(slot) = bucket.get_mut(pos) {
                *slot = upserted_config.clone();
            }
        }
        None => bucket.push(upserted_config.clone()),
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
        .and_then(|wrapper| wrapper.0.get(&profile_acquirer_id))
        .and_then(|bucket| bucket.iter().find(|cfg| cfg.network == target_network))
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get updated acquirer config after DB update")?;

    let response = profile_acquirer::ProfileAcquirerResponse::from((
        profile_acquirer_id,
        &profile_id,
        final_acquirer_config,
    ));

    Ok(api::ApplicationResponse::Json(response))
}
