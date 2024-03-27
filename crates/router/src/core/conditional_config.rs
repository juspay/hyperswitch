use api_models::{
    conditional_configs::{DecisionManager, DecisionManagerRecord, DecisionManagerResponse},
    routing,
};
use common_utils::ext_traits::{Encode, StringExt, ValueExt};
use diesel_models::configs;
use error_stack::ResultExt;
use euclid::frontend::ast;

use super::routing::helpers::{
    get_payment_config_routing_id, update_merchant_active_algorithm_ref,
};
use crate::{
    core::errors::{self, RouterResponse},
    routes::AppState,
    services::api as service_api,
    types::domain,
    utils::OptionExt,
};

pub async fn upsert_conditional_config(
    state: AppState,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
    request: DecisionManager,
) -> RouterResponse<DecisionManagerRecord> {
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
    let mut algo_id: routing::RoutingAlgorithmRef = merchant_account
        .routing_algorithm
        .clone()
        .map(|val| val.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the routing algorithm")?
        .unwrap_or_default();

    let key = get_payment_config_routing_id(merchant_account.merchant_id.as_str());
    let read_config_key = db.find_config_by_key(&key).await;

    ast::lowering::lower_program(prog.clone())
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

            algo_id.update_conditional_config_id(key);
            update_merchant_active_algorithm_ref(db, &key_store, algo_id)
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

            algo_id.update_conditional_config_id(key);
            update_merchant_active_algorithm_ref(db, &key_store, algo_id)
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

pub async fn delete_conditional_config(
    state: AppState,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
) -> RouterResponse<()> {
    let db = state.store.as_ref();
    let key = get_payment_config_routing_id(&merchant_account.merchant_id);
    let mut algo_id: routing::RoutingAlgorithmRef = merchant_account
        .routing_algorithm
        .clone()
        .map(|value| value.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the conditional_config algorithm")?
        .unwrap_or_default();
    algo_id.config_algo_id = None;
    update_merchant_active_algorithm_ref(db, &key_store, algo_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update deleted algorithm ref")?;

    db.delete_config_by_key(&key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete routing config from DB")?;
    Ok(service_api::ApplicationResponse::StatusOk)
}

pub async fn retrieve_conditional_config(
    state: AppState,
    merchant_account: domain::MerchantAccount,
) -> RouterResponse<DecisionManagerResponse> {
    let db = state.store.as_ref();
    let algorithm_id = get_payment_config_routing_id(merchant_account.merchant_id.as_str());
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
