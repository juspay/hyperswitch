use api_models::profile_acquirer;
use common_utils::types::keymanager::KeyManagerState;
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
    merchant_context: domain::MerchantContext,
) -> RouterResponse<profile_acquirer::ProfileAcquirerResponse> {
    let db = state.store.as_ref();
    let profile_acquirer_id = common_utils::generate_profile_acquirer_id_of_default_length();
    let key_manager_state: KeyManagerState = (&state).into();
    let merchant_key_store = merchant_context.get_processor_merchant_key_store();

    let mut business_profile = db
        .find_business_profile_by_profile_id(
            &key_manager_state,
            merchant_key_store,
            &request.profile_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ProfileNotFound {
            id: request.profile_id.get_string_repr().to_owned(),
        })?;

    let incoming_acquirer_config = common_types::domain::AcquirerConfig {
        acquirer_assigned_merchant_id: request.acquirer_assigned_merchant_id.clone(),
        merchant_name: request.merchant_name.clone(),
        merchant_country_code: request.merchant_country_code,
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
        .update_profile_by_profile_id(
            &key_manager_state,
            merchant_key_store,
            business_profile,
            profile_update,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update business profile with new acquirer config")?;

    let updated_acquire_details = updated_business_profile
        .acquirer_config_map
        .as_ref()
        .and_then(|acquirer_configs_wrapper| acquirer_configs_wrapper.0.get(&profile_acquirer_id))
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get updated acquirer config")?;

    let response = profile_acquirer::ProfileAcquirerResponse {
        profile_acquirer_id,
        profile_id: request.profile_id.clone(),
        acquirer_assigned_merchant_id: updated_acquire_details
            .acquirer_assigned_merchant_id
            .clone(),
        merchant_name: updated_acquire_details.merchant_name.clone(),
        merchant_country_code: updated_acquire_details.merchant_country_code,
        network: updated_acquire_details.network.clone(),
        acquirer_bin: updated_acquire_details.acquirer_bin.clone(),
        acquirer_ica: updated_acquire_details.acquirer_ica.clone(),
        acquirer_fraud_rate: updated_acquire_details.acquirer_fraud_rate,
    };

    Ok(api::ApplicationResponse::Json(response))
}
