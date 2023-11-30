use api_models::{
    routing::{self},
    surcharge_decision_configs::{
        SurchargeDecisionConfigReq, SurchargeDecisionManagerRecord,
        SurchargeDecisionManagerResponse,
    },
};
use common_utils::ext_traits::{StringExt, ValueExt};
use diesel_models::configs;
use error_stack::{IntoReport, ResultExt};
use euclid::frontend::ast;

use super::routing::helpers::{
    get_payment_method_surcharge_routing_id, update_merchant_active_algorithm_ref,
};
use crate::{
    core::errors::{self, RouterResponse},
    routes::AppState,
    services::api as service_api,
    types::domain,
    utils::{self, OptionExt},
};

pub async fn upsert_surcharge_decision_config(
    state: AppState,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
    request: SurchargeDecisionConfigReq,
) -> RouterResponse<SurchargeDecisionManagerRecord> {
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

    let key = get_payment_method_surcharge_routing_id(merchant_account.merchant_id.as_str());
    let read_config_key = db.find_config_by_key(&key).await;

    ast::lowering::lower_program(program.clone())
        .into_report()
        .change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: "Invalid Request Data".to_string(),
        })
        .attach_printable("The Request has an Invalid Comparison")?;

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

            let serialize_updated_str =
                utils::Encode::<SurchargeDecisionManagerRecord>::encode_to_string_of_json(
                    &new_algo,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to serialize config to string")?;

            let updated_config = configs::ConfigUpdate::Update {
                config: Some(serialize_updated_str),
            };

            db.update_config_by_key(&key, updated_config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error serializing the config")?;

            algo_id.update_surcharge_config_id(key);
            update_merchant_active_algorithm_ref(db, &key_store, algo_id)
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

            let serialized_str =
                utils::Encode::<SurchargeDecisionManagerRecord>::encode_to_string_of_json(&new_rec)
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

            algo_id.update_surcharge_config_id(key);
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

pub async fn delete_surcharge_decision_config(
    state: AppState,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
) -> RouterResponse<()> {
    let db = state.store.as_ref();
    let key = get_payment_method_surcharge_routing_id(&merchant_account.merchant_id);
    let mut algo_id: routing::RoutingAlgorithmRef = merchant_account
        .routing_algorithm
        .clone()
        .map(|value| value.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the surcharge conditional_config algorithm")?
        .unwrap_or_default();
    algo_id.surcharge_config_algo_id = None;
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

pub async fn retrieve_surcharge_decision_config(
    state: AppState,
    merchant_account: domain::MerchantAccount,
) -> RouterResponse<SurchargeDecisionManagerResponse> {
    let db = state.store.as_ref();
    let algorithm_id =
        get_payment_method_surcharge_routing_id(merchant_account.merchant_id.as_str());
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
