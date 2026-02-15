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

    let incoming_acquirer_config = common_types::domain::AcquirerConfig {
        acquirer_assigned_merchant_id: request.acquirer_assigned_merchant_id.clone(),
        merchant_name: request.merchant_name.clone(),
        network: request.network.clone(),
        acquirer_bin: request.acquirer_bin.clone(),
        acquirer_ica: request.acquirer_ica.clone(),
        acquirer_fraud_rate: request.acquirer_fraud_rate,
    };

    // Check for duplicates before proceeding

    business_profile
        .acquirer_config_map
        .as_ref()
        .map_or(Ok(()), |configs_wrapper| {
            match configs_wrapper.0.values().any(|existing_config| existing_config == &incoming_acquirer_config) {
                true => Err(error_stack::report!(
                    errors::ApiErrorResponse::GenericDuplicateError {
                        message: format!(
                            "Duplicate acquirer configuration found for profile_id: {}. Conflicting configuration: {:?}",
                            request.profile_id.get_string_repr(),
                            incoming_acquirer_config
                        ),
                    }
                )),
                false => Ok(()),
            }
        })?;

    // Get a mutable reference to the HashMap inside AcquirerConfigMap,
    // initializing if it's None or the inner HashMap is not present.
    let configs_map = &mut business_profile
        .acquirer_config_map
        .get_or_insert_with(|| {
            common_types::domain::AcquirerConfigMap(std::collections::HashMap::new())
        })
        .0;

    configs_map.insert(
        profile_acquirer_id.clone(),
        incoming_acquirer_config.clone(),
    );

    let profile_update = domain::ProfileUpdate::AcquirerConfigMapUpdate {
        acquirer_config_map: business_profile.acquirer_config_map.clone(),
    };
    let updated_business_profile = db
        .update_profile_by_profile_id(merchant_key_store, business_profile, profile_update)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update business profile with new acquirer config")?;

    let updated_acquire_details = updated_business_profile
        .acquirer_config_map
        .as_ref()
        .and_then(|acquirer_configs_wrapper| acquirer_configs_wrapper.0.get(&profile_acquirer_id))
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get updated acquirer config")?;

    let response = profile_acquirer::ProfileAcquirerResponse::from((
        profile_acquirer_id,
        &request.profile_id,
        updated_acquire_details,
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

    let mut potential_updated_config = acquirer_config_map
        .0
        .get(&profile_acquirer_id)
        .ok_or_else(|| errors::ApiErrorResponse::ProfileAcquirerNotFound {
            profile_id: profile_id.get_string_repr().to_owned(),
            profile_acquirer_id: profile_acquirer_id.get_string_repr().to_owned(),
        })?
        .clone();

    // updating value in existing acquirer config
    request
        .acquirer_assigned_merchant_id
        .map(|val| potential_updated_config.acquirer_assigned_merchant_id = val);
    request
        .merchant_name
        .map(|val| potential_updated_config.merchant_name = val);
    request
        .network
        .map(|val| potential_updated_config.network = val);
    request
        .acquirer_bin
        .map(|val| potential_updated_config.acquirer_bin = val);
    request
        .acquirer_ica
        .map(|val| potential_updated_config.acquirer_ica = Some(val.clone()));
    request
        .acquirer_fraud_rate
        .map(|val| potential_updated_config.acquirer_fraud_rate = val);

    // checking for duplicates in the acquirerConfigMap
    match acquirer_config_map
        .0
        .iter()
        .find(|(_existing_id, existing_config_val_ref)| {
        **existing_config_val_ref == potential_updated_config
        }) {
        Some((conflicting_id_of_found_item, _)) => {
            Err(error_stack::report!(errors::ApiErrorResponse::GenericDuplicateError {
            message: format!(
                "Duplicate acquirer configuration. This configuration already exists for profile_acquirer_id '{}' under profile_id '{}'.",
                conflicting_id_of_found_item.get_string_repr(),
                profile_id.get_string_repr()
            ),
        }))
    }
        None => Ok(()),
    }?;

    acquirer_config_map
        .0
        .insert(profile_acquirer_id.clone(), potential_updated_config);

    let updated_map_for_db_update = business_profile.acquirer_config_map.clone();

    let profile_update = domain::ProfileUpdate::AcquirerConfigMapUpdate {
        acquirer_config_map: updated_map_for_db_update,
    };

    let updated_business_profile = db
        .update_profile_by_profile_id(merchant_key_store, business_profile, profile_update)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update business profile with updated acquirer config")?;

    let final_acquirer_details = updated_business_profile
        .acquirer_config_map
        .as_ref()
        .and_then(|configs_wrapper| configs_wrapper.0.get(&profile_acquirer_id))
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get updated acquirer config after DB update")?;

    let response = profile_acquirer::ProfileAcquirerResponse::from((
        profile_acquirer_id,
        &profile_id,
        final_acquirer_details,
    ));

    Ok(api::ApplicationResponse::Json(response))
}
