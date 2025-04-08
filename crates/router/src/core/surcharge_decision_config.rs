use api_models::{
    enums,
    routing::{self, RoutingKind},
    surcharge_decision_configs::{
        SurchargeConfigResponse, SurchargeDecisionConfigReq, SurchargeDecisionManagerConfig,
        SurchargeDecisionManagerRecord, SurchargeDecisionManagerReq,
        SurchargeDecisionManagerResponse, SurchargeRecord,
    },
};
use common_utils::{
    ext_traits::{OptionExt, StringExt, ValueExt},
    id_type,
    id_type::SurchargeRoutingId,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::business_profile::ProfileUpdate;

use super::{errors::StorageErrorExt, utils};
use crate::{
    core::errors::{self, RouterResponse},
    routes::SessionState,
    services::api as service_api,
    types::{
        domain,
        transformers::{ForeignInto, ForeignTryFrom},
    },
};

#[cfg(feature = "v1")]
pub async fn upsert_surcharge_decision_config(
    state: SessionState,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
    request: SurchargeDecisionConfigReq,
) -> RouterResponse<SurchargeDecisionManagerRecord> {
    use common_utils::ext_traits::{Encode, ValueExt};
    use diesel_models::configs;
    use storage_impl::redis::cache;

    use super::routing::helpers::update_merchant_active_algorithm_ref;

    let db = state.store.as_ref();
    let name = request.name;

    let program = request
        .algorithm
        .get_required_value("algorithm")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "algorithm",
        })
        .attach_printable("Program for config not given")?;
    let merchant_surcharge_configs = request.merchant_surcharge_configs;

    let timestamp = common_utils::date_time::now_unix_timestamp();
    let mut algo_id: routing::RoutingAlgorithmRef = merchant_account
        .routing_algorithm
        .clone()
        .map(|val| val.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the routing algorithm")?
        .unwrap_or_default();

    let key = merchant_account
        .get_id()
        .get_payment_method_surcharge_routing_id();
    let read_config_key = db.find_config_by_key(&key).await;

    euclid::frontend::ast::lowering::lower_program(program.clone())
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid Request Data".to_string(),
        })
        .attach_printable("The Request has an Invalid Comparison")?;
    let surcharge_cache_key = merchant_account.get_id().get_surcharge_dsk_key();
    match read_config_key {
        Ok(config) => {
            let previous_record: SurchargeDecisionManagerRecord = config
                .config
                .parse_struct("SurchargeDecisionManagerRecord")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("The Payment Config Key Not Found")?;

            let new_algo = SurchargeDecisionManagerRecord {
                name: name.unwrap_or(previous_record.name),
                algorithm: program,
                modified_at: timestamp,
                created_at: previous_record.created_at,
                merchant_surcharge_configs,
            };

            let serialize_updated_str = new_algo
                .encode_to_string_of_json()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to serialize config to string")?;

            let updated_config = configs::ConfigUpdate::Update {
                config: Some(serialize_updated_str),
            };

            db.update_config_by_key(&key, updated_config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error serializing the config")?;

            algo_id.update_surcharge_config_id(key.clone());
            let config_key = cache::CacheKind::Surcharge(surcharge_cache_key.into());
            update_merchant_active_algorithm_ref(&state, &key_store, config_key, algo_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update routing algorithm ref")?;

            Ok(service_api::ApplicationResponse::Json(new_algo))
        }
        Err(e) if e.current_context().is_db_not_found() => {
            let new_rec = SurchargeDecisionManagerRecord {
                name: name
                    .get_required_value("name")
                    .change_context(errors::ApiErrorResponse::MissingRequiredField {
                        field_name: "name",
                    })
                    .attach_printable("name of the config not found")?,
                algorithm: program,
                merchant_surcharge_configs,
                modified_at: timestamp,
                created_at: timestamp,
            };

            let serialized_str = new_rec
                .encode_to_string_of_json()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error serializing the config")?;
            let new_config = configs::ConfigNew {
                key: key.clone(),
                config: serialized_str,
            };

            db.insert_config(new_config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error fetching the config")?;

            algo_id.update_surcharge_config_id(key.clone());
            let config_key = cache::CacheKind::Surcharge(surcharge_cache_key.into());
            update_merchant_active_algorithm_ref(&state, &key_store, config_key, algo_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update routing algorithm ref")?;

            Ok(service_api::ApplicationResponse::Json(new_rec))
        }
        Err(e) => Err(e)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error fetching payment config"),
    }
}

#[cfg(feature = "v2")]
pub async fn upsert_surcharge_decision_config(
    _state: SessionState,
    _key_store: domain::MerchantKeyStore,
    _merchant_account: domain::MerchantAccount,
    _request: SurchargeDecisionConfigReq,
) -> RouterResponse<SurchargeDecisionManagerRecord> {
    todo!();
}

#[cfg(feature = "v1")]
pub async fn delete_surcharge_decision_config(
    state: SessionState,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
) -> RouterResponse<()> {
    use common_utils::ext_traits::ValueExt;
    use storage_impl::redis::cache;

    use super::routing::helpers::update_merchant_active_algorithm_ref;

    let db = state.store.as_ref();
    let key = merchant_account
        .get_id()
        .get_payment_method_surcharge_routing_id();
    let mut algo_id: routing::RoutingAlgorithmRef = merchant_account
        .routing_algorithm
        .clone()
        .map(|value| value.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the surcharge conditional_config algorithm")?
        .unwrap_or_default();
    algo_id.surcharge_config_algo_id = None;
    let surcharge_cache_key = merchant_account.get_id().get_surcharge_dsk_key();
    let config_key = cache::CacheKind::Surcharge(surcharge_cache_key.into());
    update_merchant_active_algorithm_ref(&state, &key_store, config_key, algo_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update deleted algorithm ref")?;

    db.delete_config_by_key(&key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete routing config from DB")?;
    Ok(service_api::ApplicationResponse::StatusOk)
}

#[cfg(feature = "v2")]
pub async fn delete_surcharge_decision_config(
    _state: SessionState,
    _key_store: domain::MerchantKeyStore,
    _merchant_account: domain::MerchantAccount,
) -> RouterResponse<()> {
    todo!()
}

pub async fn retrieve_surcharge_decision_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
) -> RouterResponse<SurchargeDecisionManagerResponse> {
    let db = state.store.as_ref();
    let algorithm_id = merchant_account
        .get_id()
        .get_payment_method_surcharge_routing_id();
    let algo_config = db
        .find_config_by_key(&algorithm_id)
        .await
        .change_context(errors::ApiErrorResponse::ResourceIdNotFound)
        .attach_printable("The surcharge conditional config was not found in the DB")?;
    let record: SurchargeDecisionManagerRecord = algo_config
        .config
        .parse_struct("SurchargeDecisionConfigsRecord")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("The Surcharge Decision Config Record was not found")?;
    Ok(service_api::ApplicationResponse::Json(record))
}

pub async fn list_surcharge_decision_configs(
    state: SessionState,
    profile_id: Option<id_type::ProfileId>,
    limit: i64,
    offset: i64,
    algorithm_type: enums::AlgorithmType,
) -> RouterResponse<RoutingKind> {
    let profile_id = profile_id.get_required_value("profile_id").change_context(
        errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        },
    )?;

    let db = state.store.as_ref();

    let records = db
        .list_routing_algorithm_metadata_by_profile_id_algorithm_type(
            &profile_id,
            limit,
            offset,
            algorithm_type,
        )
        .await
        .change_context(errors::ApiErrorResponse::ResourceIdNotFound)
        .attach_printable("Surcharge Details could not be found in DB")?;

    let result = records
        .into_iter()
        .map(ForeignInto::foreign_into)
        .collect::<Vec<_>>();

    Ok(service_api::ApplicationResponse::Json(
        RoutingKind::RoutingAlgorithm(result),
    ))
}

pub async fn add_surcharge_decision_config(
    state: SessionState,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
    profile_id: Option<id_type::ProfileId>,
    request: SurchargeDecisionManagerReq,
    transaction_type: enums::TransactionType,
    algorithm_type: enums::AlgorithmType,
) -> RouterResponse<SurchargeRecord> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let name = request.name;

    let description = request.description;

    let surcharge_algorithm_data = SurchargeDecisionManagerConfig {
        merchant_surcharge_configs: request.merchant_surcharge_configs.clone(),
        algorithm: request.algorithm,
    };

    let algorithm_id = common_utils::generate_routing_id_of_default_length();

    let profile_id = profile_id.ok_or_else(|| errors::ApiErrorResponse::MissingRequiredField {
        field_name: "profile_id",
    })?;

    let business_profile = utils::validate_and_get_business_profile(
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

    utils::validate_profile_id_from_auth_layer(Some(profile_id.clone()), &business_profile)?;

    let timestamp = common_utils::date_time::now();

    let algo = diesel_models::routing_algorithm::RoutingAlgorithm {
        algorithm_id: algorithm_id.clone(),
        profile_id,
        merchant_id: merchant_account.get_id().to_owned(),
        name: name.clone(),
        description: description.clone(),
        kind: diesel_models::enums::RoutingAlgorithmKind::Advanced,
        algorithm_data: serde_json::json!(surcharge_algorithm_data),
        created_at: timestamp,
        modified_at: timestamp,
        algorithm_for: transaction_type,
        algorithm_type,
    };
    let record = db
        .insert_routing_algorithm(algo)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert surcharge decision config")?;

    let new_record = SurchargeRecord::foreign_try_from(record)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert record to SurchargeRecord")?;

    Ok(service_api::ApplicationResponse::Json(new_record))
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn retrieve_surcharge_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    query: routing::SurchargeRetrieveLinkQuery,
) -> RouterResponse<SurchargeConfigResponse> {
    use error_stack::report;

    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let final_response;
    let business_profile = utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&query.profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: query.profile_id.get_string_repr().to_owned(),
    })?;

    if let Some(active_surcharge_algorithm_id) = business_profile.active_surcharge_algorithm_id {
        let active_algorithm_id = active_surcharge_algorithm_id.0;
        let record = db
            .find_routing_algorithm_by_profile_id_algorithm_id(
                &query.profile_id,
                &active_algorithm_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

        final_response = SurchargeRecord::foreign_try_from(record.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert record to SurchargeRecord")?;
    } else {
        return Err(report!(errors::ApiErrorResponse::ResourceIdNotFound)
            .attach_printable("Active surcharge algorithm ID not found"));
    };

    Ok(service_api::ApplicationResponse::Json(final_response))
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn link_surcharge_decision_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    auth_profile_id: Option<id_type::ProfileId>,
    algorithm_id: SurchargeRoutingId,
) -> RouterResponse<SurchargeConfigResponse> {
    let profile_id =
        auth_profile_id.ok_or_else(|| errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        })?;

    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let business_profile = utils::validate_and_get_business_profile(
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

    let record = db
        .find_surcharge_algorithm_by_profile_id_algorithm_id(&profile_id, &algorithm_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let profile_update = ProfileUpdate::ActiveSurchargeIdUpdate {
        active_surcharge_algorithm_id: Some(algorithm_id),
    };

    let updated_profile = db
        .update_profile_by_profile_id(
            key_manager_state,
            &key_store,
            business_profile,
            profile_update,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update active surcharge algorithm ID")?;

    let response = updated_profile.active_surcharge_algorithm_id.map_or_else(
        || {
            Err(errors::ApiErrorResponse::ResourceIdNotFound)
                .attach_printable("Active surcharge algorithm ID not found")
        },
        |_| {
            SurchargeRecord::foreign_try_from(record.clone())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to convert record to SurchargeRecord")
        },
    )?;

    Ok(service_api::ApplicationResponse::Json(response))
}

impl ForeignTryFrom<diesel_models::routing_algorithm::RoutingAlgorithm> for SurchargeRecord {
    type Error = error_stack::Report<common_utils::errors::ParsingError>;

    fn foreign_try_from(
        record: diesel_models::routing_algorithm::RoutingAlgorithm,
    ) -> Result<Self, Self::Error> {
        let algorithm_data: SurchargeDecisionManagerConfig = record
            .algorithm_data
            .parse_value("SurchargeDecisionManagerConfig")
            .attach_printable("Failed to deserialise surcharge config")?;

        Ok(Self {
            name: record.name,
            algorithm_id: SurchargeRoutingId(record.algorithm_id),
            merchant_surcharge_configs: algorithm_data.merchant_surcharge_configs,
            algorithm: algorithm_data.algorithm,
            description: record.description,
            created_at: record.created_at,
            modified_at: record.modified_at,
        })
    }
}
