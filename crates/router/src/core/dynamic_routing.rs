pub mod helpers;

use api_models::{routing, routing as routing_types};
use diesel_models::routing_algorithm::RoutingAlgorithm;
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use router_env::logger;
use router_env::metrics::add_attributes;
#[cfg(feature = "v1")]
use storage_impl::redis::cache;

#[cfg(feature = "v1")]
use crate::utils::ValueExt;
#[cfg(feature = "v2")]
use crate::{core::admin, utils::ValueExt};
use crate::{
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        metrics, utils as core_utils,
    },
    routes::SessionState,
    services::api as service_api,
    types::{domain, transformers::ForeignInto},
    utils::OptionExt,
};

#[cfg(feature = "v1")]
pub async fn toggle_success_based_routing(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    feature_to_enable: routing::SuccessBasedRoutingFeatures,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_CREATE_REQUEST_RECEIVED.add(
        &metrics::CONTEXT,
        1,
        &add_attributes([("profile_id", profile_id.get_string_repr().to_owned())]),
    );
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let business_profile: domain::Profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: profile_id.get_string_repr().to_owned(),
    })?;

    let mut success_based_dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef =
        business_profile
            .dynamic_routing_algorithm
            .clone()
            .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to deserialize dynamic routing algorithm ref from business profile",
            )?
            .unwrap_or_default();

    match feature_to_enable {
        routing::SuccessBasedRoutingFeatures::Metrics
        | routing::SuccessBasedRoutingFeatures::DynamicConnectorSelection => {
            if let Some(ref mut algo_with_timestamp) =
                success_based_dynamic_routing_algo_ref.success_based_algorithm
            {
                match algo_with_timestamp
                    .algorithm_id_with_timestamp
                    .algorithm_id
                    .clone()
                {
                    Some(algorithm_id) => {
                        // algorithm is already present in profile
                        if algo_with_timestamp.enabled_feature == feature_to_enable {
                            // algorithm already has the required feature
                            Err(errors::ApiErrorResponse::PreconditionFailed {
                                message: "Success rate based routing is already enabled"
                                    .to_string(),
                            })?
                        } else {
                            // enable the requested feature for the algorithm
                            algo_with_timestamp.update_enabled_features(feature_to_enable);
                            let record = db
                                .find_routing_algorithm_by_profile_id_algorithm_id(
                                    business_profile.get_id(),
                                    &algorithm_id,
                                )
                                .await
                                .to_not_found_response(
                                    errors::ApiErrorResponse::ResourceIdNotFound,
                                )?;
                            let response = record.foreign_into();
                            helpers::update_business_profile_active_dynamic_algorithm_ref(
                                db,
                                key_manager_state,
                                &key_store,
                                business_profile,
                                success_based_dynamic_routing_algo_ref,
                            )
                            .await?;

                            metrics::ROUTING_CREATE_SUCCESS_RESPONSE.add(
                                &metrics::CONTEXT,
                                1,
                                &add_attributes([(
                                    "profile_id",
                                    profile_id.get_string_repr().to_owned(),
                                )]),
                            );
                            Ok(service_api::ApplicationResponse::Json(response))
                        }
                    }
                    None => {
                        // algorithm isn't present in profile
                        helpers::default_success_based_routing_setup(
                            &state,
                            key_store,
                            business_profile,
                            feature_to_enable,
                            merchant_account.get_id().to_owned(),
                            success_based_dynamic_routing_algo_ref,
                        )
                        .await
                    }
                }
            } else {
                // algorithm isn't present in profile
                helpers::default_success_based_routing_setup(
                    &state,
                    key_store,
                    business_profile,
                    feature_to_enable,
                    merchant_account.get_id().to_owned(),
                    success_based_dynamic_routing_algo_ref,
                )
                .await
            }
        }
        routing::SuccessBasedRoutingFeatures::None => {
            // disable success based routing for the requested profile
            let timestamp = common_utils::date_time::now_unix_timestamp();
            match success_based_dynamic_routing_algo_ref.success_based_algorithm {
                Some(algorithm_ref) => {
                    if let Some(algorithm_id) =
                        algorithm_ref.algorithm_id_with_timestamp.algorithm_id
                    {
                        let dynamic_routing_algorithm = routing_types::DynamicRoutingAlgorithmRef {
                            success_based_algorithm: Some(routing::SuccessBasedAlgorithm {
                                algorithm_id_with_timestamp:
                                    routing_types::DynamicAlgorithmWithTimestamp {
                                        algorithm_id: None,
                                        timestamp,
                                    },
                                enabled_feature: routing::SuccessBasedRoutingFeatures::None,
                            }),
                        };

                        // redact cache for success based routing configs
                        let cache_key = format!(
                            "{}_{}",
                            business_profile.get_id().get_string_repr(),
                            algorithm_id.get_string_repr()
                        );
                        let cache_entries_to_redact =
                            vec![cache::CacheKind::SuccessBasedDynamicRoutingCache(
                                cache_key.into(),
                            )];
                        let _ = cache::publish_into_redact_channel(
                            state.store.get_cache_store().as_ref(),
                            cache_entries_to_redact,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("unable to publish into the redact channel for evicting the success based routing config cache")?;

                        let record = db
                            .find_routing_algorithm_by_profile_id_algorithm_id(
                                business_profile.get_id(),
                                &algorithm_id,
                            )
                            .await
                            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
                        let response = record.foreign_into();
                        helpers::update_business_profile_active_dynamic_algorithm_ref(
                            db,
                            key_manager_state,
                            &key_store,
                            business_profile,
                            dynamic_routing_algorithm,
                        )
                        .await?;

                        metrics::ROUTING_UNLINK_CONFIG_SUCCESS_RESPONSE.add(
                            &metrics::CONTEXT,
                            1,
                            &add_attributes([(
                                "profile_id",
                                profile_id.get_string_repr().to_owned(),
                            )]),
                        );

                        Ok(service_api::ApplicationResponse::Json(response))
                    } else {
                        Err(errors::ApiErrorResponse::PreconditionFailed {
                            message: "Algorithm is already inactive".to_string(),
                        })?
                    }
                }
                None => Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Success rate based routing is already disabled".to_string(),
                })?,
            }
        }
    }
}

#[cfg(feature = "v1")]
pub async fn success_based_routing_update_configs(
    state: SessionState,
    request: routing_types::SuccessBasedRoutingConfig,
    algorithm_id: common_utils::id_type::RoutingId,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE.add(
        &metrics::CONTEXT,
        1,
        &add_attributes([("profile_id", profile_id.get_string_repr().to_owned())]),
    );
    let db = state.store.as_ref();

    let dynamic_routing_algo_to_update = db
        .find_routing_algorithm_by_profile_id_algorithm_id(&profile_id, &algorithm_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let mut config_to_update: routing::SuccessBasedRoutingConfig = dynamic_routing_algo_to_update
        .algorithm_data
        .parse_value::<routing::SuccessBasedRoutingConfig>("SuccessBasedRoutingConfig")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to deserialize algorithm data from routing table into SuccessBasedRoutingConfig")?;

    config_to_update.update(request);

    let updated_algorithm_id = common_utils::generate_routing_id_of_default_length();
    let timestamp = common_utils::date_time::now();
    let algo = RoutingAlgorithm {
        algorithm_id: updated_algorithm_id,
        profile_id: dynamic_routing_algo_to_update.profile_id,
        merchant_id: dynamic_routing_algo_to_update.merchant_id,
        name: dynamic_routing_algo_to_update.name,
        description: dynamic_routing_algo_to_update.description,
        kind: dynamic_routing_algo_to_update.kind,
        algorithm_data: serde_json::json!(config_to_update),
        created_at: timestamp,
        modified_at: timestamp,
        algorithm_for: dynamic_routing_algo_to_update.algorithm_for,
    };
    let record = db
        .insert_routing_algorithm(algo)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to insert record in routing algorithm table")?;

    // redact cache for success based routing configs
    let cache_key = format!(
        "{}_{}",
        profile_id.get_string_repr(),
        algorithm_id.get_string_repr()
    );
    let cache_entries_to_redact = vec![cache::CacheKind::SuccessBasedDynamicRoutingCache(
        cache_key.into(),
    )];
    let _ = cache::publish_into_redact_channel(
        state.store.get_cache_store().as_ref(),
        cache_entries_to_redact,
    )
    .await
    .map_err(|e| logger::error!("unable to publish into the redact channel for evicting the success based routing config cache {e:?}"));

    let new_record = record.foreign_into();

    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE_SUCCESS_RESPONSE.add(
        &metrics::CONTEXT,
        1,
        &add_attributes([("profile_id", profile_id.get_string_repr().to_owned())]),
    );
    Ok(service_api::ApplicationResponse::Json(new_record))
}
