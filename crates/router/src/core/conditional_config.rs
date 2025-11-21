#[cfg(feature = "v2")]
use api_models::conditional_configs::DecisionManagerRequest;
use api_models::conditional_configs::{
    DecisionManager, DecisionManagerRecord, DecisionManagerResponse,
};
use common_utils::ext_traits::StringExt;
#[cfg(feature = "v2")]
use common_utils::types::keymanager::KeyManagerState;
use error_stack::ResultExt;

use crate::{
    core::errors::{self, RouterResponse},
    routes::SessionState,
    services::api as service_api,
    types::domain,
};
#[cfg(feature = "v2")]
pub async fn upsert_conditional_config(
    state: SessionState,
    key_store: domain::MerchantKeyStore,
    request: DecisionManagerRequest,
    profile: domain::Profile,
) -> RouterResponse<common_types::payments::DecisionManagerRecord> {
    use common_utils::ext_traits::OptionExt;

    let key_manager_state: &KeyManagerState = &(&state).into();
    let db = &*state.store;
    let name = request.name;
    let program = request.program;
    let timestamp = common_utils::date_time::now_unix_timestamp();

    euclid::frontend::ast::lowering::lower_program(program.clone())
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid Request Data".to_string(),
        })
        .attach_printable("The Request has an Invalid Comparison")?;

    let decision_manager_record = common_types::payments::DecisionManagerRecord {
        name,
        program,
        created_at: timestamp,
    };

    let business_profile_update = domain::ProfileUpdate::DecisionManagerRecordUpdate {
        three_ds_decision_manager_config: decision_manager_record,
    };
    let updated_profile = db
        .update_profile_by_profile_id(&key_store, profile, business_profile_update)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update decision manager record in business profile")?;

    Ok(service_api::ApplicationResponse::Json(
        updated_profile
            .three_ds_decision_manager_config
            .clone()
            .get_required_value("three_ds_decision_manager_config")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to get updated decision manager record in business profile",
            )?,
    ))
}

#[cfg(feature = "v1")]
pub async fn upsert_conditional_config(
    state: SessionState,
    platform: domain::Platform,
    request: DecisionManager,
) -> RouterResponse<DecisionManagerRecord> {
    use common_utils::ext_traits::{Encode, OptionExt, ValueExt};
    use diesel_models::configs;
    use storage_impl::redis::cache;

    use super::routing::helpers::update_merchant_active_algorithm_ref;

    let db = state.store.as_ref();
    let (name, prog) = match request {
        DecisionManager::DecisionManagerv0(ccr) => {
            let name = ccr.name;

            let prog = ccr
                .algorithm
                .get_required_value("algorithm")
                .change_context(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "algorithm",
                })
                .attach_printable("Algorithm for config not given")?;
            (name, prog)
        }
        DecisionManager::DecisionManagerv1(dmr) => {
            let name = dmr.name;

            let prog = dmr
                .program
                .get_required_value("program")
                .change_context(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "program",
                })
                .attach_printable("Program for config not given")?;
            (name, prog)
        }
    };
    let timestamp = common_utils::date_time::now_unix_timestamp();
    let mut algo_id: api_models::routing::RoutingAlgorithmRef = platform
        .get_processor()
        .get_account()
        .routing_algorithm
        .clone()
        .map(|val| val.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the routing algorithm")?
        .unwrap_or_default();

    let key = platform
        .get_processor()
        .get_account()
        .get_id()
        .get_payment_config_routing_id();
    let read_config_key = db.find_config_by_key(&key).await;

    euclid::frontend::ast::lowering::lower_program(prog.clone())
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid Request Data".to_string(),
        })
        .attach_printable("The Request has an Invalid Comparison")?;

    match read_config_key {
        Ok(config) => {
            let previous_record: DecisionManagerRecord = config
                .config
                .parse_struct("DecisionManagerRecord")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("The Payment Config Key Not Found")?;

            let new_algo = DecisionManagerRecord {
                name: previous_record.name,
                program: prog,
                modified_at: timestamp,
                created_at: previous_record.created_at,
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

            algo_id.update_conditional_config_id(key.clone());
            let config_key = cache::CacheKind::DecisionManager(key.into());
            update_merchant_active_algorithm_ref(
                &state,
                platform.get_processor().get_key_store(),
                config_key,
                algo_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update routing algorithm ref")?;

            Ok(service_api::ApplicationResponse::Json(new_algo))
        }
        Err(e) if e.current_context().is_db_not_found() => {
            let new_rec = DecisionManagerRecord {
                name: name
                    .get_required_value("name")
                    .change_context(errors::ApiErrorResponse::MissingRequiredField {
                        field_name: "name",
                    })
                    .attach_printable("name of the config not found")?,
                program: prog,
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

            algo_id.update_conditional_config_id(key.clone());
            let config_key = cache::CacheKind::DecisionManager(key.into());
            update_merchant_active_algorithm_ref(
                &state,
                platform.get_processor().get_key_store(),
                config_key,
                algo_id,
            )
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
pub async fn delete_conditional_config(
    _state: SessionState,
    _platform: domain::Platform,
) -> RouterResponse<()> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn delete_conditional_config(
    state: SessionState,
    platform: domain::Platform,
) -> RouterResponse<()> {
    use common_utils::ext_traits::ValueExt;
    use storage_impl::redis::cache;

    use super::routing::helpers::update_merchant_active_algorithm_ref;

    let db = state.store.as_ref();
    let key = platform
        .get_processor()
        .get_account()
        .get_id()
        .get_payment_config_routing_id();
    let mut algo_id: api_models::routing::RoutingAlgorithmRef = platform
        .get_processor()
        .get_account()
        .routing_algorithm
        .clone()
        .map(|value| value.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the conditional_config algorithm")?
        .unwrap_or_default();
    algo_id.config_algo_id = None;
    let config_key = cache::CacheKind::DecisionManager(key.clone().into());
    update_merchant_active_algorithm_ref(
        &state,
        platform.get_processor().get_key_store(),
        config_key,
        algo_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update deleted algorithm ref")?;

    db.delete_config_by_key(&key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete routing config from DB")?;
    Ok(service_api::ApplicationResponse::StatusOk)
}

#[cfg(feature = "v1")]
pub async fn retrieve_conditional_config(
    state: SessionState,
    platform: domain::Platform,
) -> RouterResponse<DecisionManagerResponse> {
    let db = state.store.as_ref();
    let algorithm_id = platform
        .get_processor()
        .get_account()
        .get_id()
        .get_payment_config_routing_id();
    let algo_config = db
        .find_config_by_key(&algorithm_id)
        .await
        .change_context(errors::ApiErrorResponse::ResourceIdNotFound)
        .attach_printable("The conditional config was not found in the DB")?;
    let record: DecisionManagerRecord = algo_config
        .config
        .parse_struct("ConditionalConfigRecord")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("The Conditional Config Record was not found")?;

    let response = DecisionManagerRecord {
        name: record.name,
        program: record.program,
        created_at: record.created_at,
        modified_at: record.modified_at,
    };
    Ok(service_api::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn retrieve_conditional_config(
    state: SessionState,
    key_store: domain::MerchantKeyStore,
    profile: domain::Profile,
) -> RouterResponse<common_types::payments::DecisionManagerResponse> {
    let db = state.store.as_ref();
    let key_manager_state: &KeyManagerState = &(&state).into();
    let profile_id = profile.get_id();

    let record = profile
        .three_ds_decision_manager_config
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("The Conditional Config Record was not found")?;

    let response = common_types::payments::DecisionManagerRecord {
        name: record.name,
        program: record.program,
        created_at: record.created_at,
    };
    Ok(service_api::ApplicationResponse::Json(response))
}
