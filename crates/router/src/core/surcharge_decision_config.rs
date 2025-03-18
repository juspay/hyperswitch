use api_models::{
    routing::{self, RoutingKind},
    surcharge_decision_configs::{
        SurchargeDecisionConfigReq, SurchargeDecisionManagerConfig, SurchargeDecisionManagerRecord,
        SurchargeDecisionManagerResponse,
    },
};
use common_utils::ext_traits::ValueExt;
use common_enums::AlgorithmType;
use common_utils::{
    ext_traits::{OptionExt, StringExt},
    id_type,
};
use error_stack::ResultExt;

use super::{errors::StorageErrorExt, utils};
use crate::{
    core::errors::{self, RouterResponse},
    routes::SessionState,
    services::api as service_api,
    types::{domain, transformers::ForeignInto},
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
    let mut algo_id: api_models::routing::RoutingAlgorithmRef = merchant_account
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
    let mut algo_id: api_models::routing::RoutingAlgorithmRef = merchant_account
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
    profile_id: id_type::ProfileId,
    limit: i64,
    offset: i64,
) -> RouterResponse<RoutingKind> {
    let db = state.store.as_ref();

    let records = db
        .list_routing_algorithm_metadata_by_profile_id_algorithm_type(
            &profile_id,
            limit,
            offset,
            AlgorithmType::Surcharge,
        )
        .await
        .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

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
    profile_id: id_type::ProfileId,
    request: SurchargeDecisionConfigReq,
    transaction_type: api_models::enums::TransactionType,
    algorithm_type: AlgorithmType,
) -> RouterResponse<SurchargeDecisionManagerRecord> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let name = request
        .name
        .get_required_value("name")
        .change_context(errors::ApiErrorResponse::MissingRequiredField { field_name: "name" })
        .attach_printable("Name of config not given")?;

    let description = request
        .description
        .get_required_value("description")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "description",
        })
        .attach_printable("Description of config not given")?;

    let surcharge_algorithm_data = SurchargeDecisionManagerConfig {
        merchant_surcharge_configs: request.merchant_surcharge_configs.clone(),
        algorithm: request.algorithm
            .get_required_value("algorithm")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "algorithm",
            })
            .attach_printable("Algorithm of config not given")?,
    };

    let algorithm_id = common_utils::generate_routing_id_of_default_length();

    let business_profile = utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    utils::validate_profile_id_from_auth_layer(Some(profile_id.clone()), &business_profile)?;

    let timestamp = common_utils::date_time::now();

    let algo = diesel_models::routing_algorithm::RoutingAlgorithm {
        algorithm_id: algorithm_id.clone(),
        profile_id,
        merchant_id: merchant_account.get_id().to_owned(),
        name: name.clone(),
        description: Some(description.clone()),
        kind: diesel_models::enums::RoutingAlgorithmKind::Advanced,
        algorithm_data: serde_json::json!(surcharge_algorithm_data),
        created_at: timestamp,
        modified_at: timestamp,
        algorithm_for: transaction_type.to_owned(),
        algorithm_type: algorithm_type.to_owned(),
    };
    let record = db
        .insert_routing_algorithm(algo)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let new_record = SurchargeDecisionManagerRecord {
        name,
        merchant_surcharge_configs: surcharge_algorithm_data.merchant_surcharge_configs,
        algorithm: surcharge_algorithm_data.algorithm,
        created_at: record.created_at.assume_utc().unix_timestamp(),
        modified_at: record.modified_at.assume_utc().unix_timestamp(),
    };

    Ok(service_api::ApplicationResponse::Json(new_record))
}

pub async fn retrieve_surcharge_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    query: routing::SurchargeRetrieveLinkQuery,
) -> RouterResponse<SurchargeDecisionManagerResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let profile_id = query.profile_id;
    let business_profile = if let Some(profile_id) = profile_id.clone() {
        utils::validate_and_get_business_profile(
            db,
            key_manager_state,
            &key_store,
            Some(&profile_id),
            merchant_account.get_id(),
        )
        .await?
        .get_required_value("Profile")
        .change_context(errors::ApiErrorResponse::ProfileNotFound { 
            id: profile_id.get_string_repr().to_owned() 
        })?
    } else {
        return Err(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        })
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        })
        .attach_printable("Profile ID is missing in the query");
    };

    let response = if let Some(active_surcharge_algorithm_id) = business_profile.active_surcharge_algorithm_id {
        let active_algorithm_id = active_surcharge_algorithm_id.0;
        let record = db
            .find_routing_algorithm_by_profile_id_algorithm_id(&profile_id.unwrap(), &active_algorithm_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

        let record_data: SurchargeDecisionManagerConfig = record
            .algorithm_data
            .parse_value("SurchargeDecisionManagerConfig")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        SurchargeDecisionManagerRecord {
            name: record.name,
            merchant_surcharge_configs: record_data.merchant_surcharge_configs,
            algorithm: record_data.algorithm,
            created_at: record.created_at.assume_utc().unix_timestamp(),
            modified_at: record.modified_at.assume_utc().unix_timestamp(),
        }
    } else {
        return Err(errors::ApiErrorResponse::ResourceIdNotFound)
            .change_context(errors::ApiErrorResponse::ResourceIdNotFound)
            .attach_printable("Active surcharge algorithm ID not found");
    };

    Ok(service_api::ApplicationResponse::Json(response))
}
