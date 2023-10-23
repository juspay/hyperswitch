pub mod helpers;
pub mod transformers;

use api_models::routing as routing_types;
#[cfg(feature = "business_profile_routing")]
use api_models::routing::{RoutingRetrieveLinkQuery, RoutingRetrieveQuery};
#[cfg(not(feature = "business_profile_routing"))]
use common_utils::ext_traits::StringExt;
#[cfg(not(feature = "business_profile_routing"))]
use diesel_models::configs;
use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "business_profile_routing")]
use hyperswitch_oss::core::utils::validate_and_get_business_profile;
#[cfg(feature = "business_profile_routing")]
use storage_models::routing_algorithm::RoutingAlgorithm;

#[cfg(feature = "business_profile_routing")]
use crate::types::transformers::{ForeignInto, ForeignTryInto};
#[cfg(not(feature = "business_profile_routing"))]
use crate::{
    consts,
    core::errors::{self, RouterResponse},
    routes::AppState,
    services::api as service_api,
    types::domain,
    utils::{self, OptionExt, ValueExt},
};

pub async fn retrieve_merchant_routing_dictionary(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    #[cfg(feature = "business_profile_routing")] query_params: RoutingRetrieveQuery,
) -> RouterResponse<routing_types::RoutingKind> {
    #[cfg(feature = "business_profile_routing")]
    {
        let routing_metadata = state
            .store
            .list_routing_algorithm_metadata_by_merchant_id(
                &merchant_account.merchant_id,
                i64::from(query_params.limit.unwrap_or_default()),
                i64::from(query_params.offset.unwrap_or_default()),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
        let result = routing_metadata
            .into_iter()
            .map(ForeignInto::foreign_into)
            .collect::<Vec<_>>();

        Ok(service_api::ApplicationResponse::Json(
            routing_types::RoutingKind::RoutingAlgorithm(result),
        ))
    }
    #[cfg(not(feature = "business_profile_routing"))]
    Ok(service_api::ApplicationResponse::Json(
        routing_types::RoutingKind::Config(
            helpers::get_merchant_routing_dictionary(
                state.store.as_ref(),
                &merchant_account.merchant_id,
            )
            .await?,
        ),
    ))
}

pub async fn create_routing_config(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    #[cfg(not(feature = "business_profile_routing"))] key_store: domain::MerchantKeyStore,
    request: routing_types::RoutingConfigRequest,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    let db = state.store.as_ref();

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

    let algorithm = request
        .algorithm
        .get_required_value("algorithm")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "algorithm",
        })
        .attach_printable("Algorithm of config not given")?;

    let algorithm_id = common_utils::generate_id(
        consts::ROUTING_CONFIG_ID_LENGTH,
        &format!("routing_{}", &merchant_account.merchant_id),
    );

    #[cfg(feature = "business_profile_routing")]
    {
        let profile_id = request
            .profile_id
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "profile_id",
            })
            .attach_printable("Profile_id not provided")?;

        validate_and_get_business_profile(
            &*db.down_cast(),
            Some(&profile_id),
            &merchant_account.merchant_id,
        )
        .await?;
        let timestamp = common_utils::date_time::now();
        let algo = RoutingAlgorithm {
            algorithm_id: algorithm_id.clone(),
            profile_id,
            merchant_id: merchant_account.merchant_id,
            name: name.clone(),
            description: Some(description.clone()),
            kind: algorithm.get_kind().foreign_into(),
            algorithm_data: serde_json::json!(algorithm),
            created_at: timestamp,
            modified_at: timestamp,
        };
        let record = db
            .insert_routing_algorithm(algo)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

        let new_record = record.foreign_into();

        Ok(service_api::ApplicationResponse::Json(new_record))
    }

    #[cfg(not(feature = "business_profile_routing"))]
    {
        let algorithm_str =
            utils::Encode::<routing_types::RoutingAlgorithm>::encode_to_string_of_json(&algorithm)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to serialize routing algorithm to string")?;

        let mut algorithm_ref: routing_types::RoutingAlgorithmRef = merchant_account
            .routing_algorithm
            .clone()
            .map(|val| val.parse_value("RoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to deserialize routing algorithm ref from merchant account")?
            .unwrap_or_default();
        let mut merchant_dictionary =
            helpers::get_merchant_routing_dictionary(db, &merchant_account.merchant_id).await?;

        utils::when(
            merchant_dictionary.records.len() >= consts::MAX_ROUTING_CONFIGS_PER_MERCHANT,
            || {
                Err(errors::ApiErrorResponse::PreconditionFailed {
            message: format!("Reached the maximum number of routing configs ({}), please delete some to create new ones", consts::MAX_ROUTING_CONFIGS_PER_MERCHANT),
        })
        .into_report()
            },
        )?;
        let timestamp = common_utils::date_time::now_unix_timestamp();
        let records_are_empty = merchant_dictionary.records.is_empty();

        let new_record = routing_types::RoutingDictionaryRecord {
            id: algorithm_id.clone(),
            name: name.clone(),
            kind: algorithm.get_kind(),
            description: description.clone(),
            created_at: timestamp,
            modified_at: timestamp,
        };
        merchant_dictionary.records.push(new_record.clone());

        let new_algorithm_config = configs::ConfigNew {
            key: algorithm_id.clone(),
            config: algorithm_str,
        };

        db.insert_config(new_algorithm_config)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to save new routing algorithm config to DB")?;

        if records_are_empty {
            merchant_dictionary.active_id = Some(algorithm_id.clone());
            algorithm_ref.update_algorithm_id(algorithm_id);
            helpers::update_merchant_active_algorithm_ref(db, &key_store, algorithm_ref).await?;
        }

        helpers::update_merchant_routing_dictionary(
            db,
            &merchant_account.merchant_id,
            merchant_dictionary,
        )
        .await?;

        Ok(service_api::ApplicationResponse::Json(new_record))
    }
}

pub async fn link_routing_config(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    #[cfg(not(feature = "business_profile_routing"))] key_store: domain::MerchantKeyStore,
    algorithm_id: String,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    let db = state.store.as_ref();
    #[cfg(feature = "business_profile_routing")]
    {
        let routing_algorithm = db
            .find_routing_algorithm_by_algorithm_id_merchant_id(
                &algorithm_id,
                &merchant_account.merchant_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

        let business_profile = validate_and_get_business_profile(
            &*db.down_cast(),
            Some(&routing_algorithm.profile_id),
            &merchant_account.merchant_id,
        )
        .await?
        .get_required_value("BusinessProfile")
        .change_context(errors::ApiErrorResponse::BusinessProfileNotFound {
            id: routing_algorithm.profile_id.clone(),
        })?;

        let mut routing_ref: routing_types::RoutingAlgorithmRef = business_profile
            .routing_algorithm
            .clone()
            .map(|val| val.parse_value("RoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to deserialize routing algorithm ref from merchant account")?
            .unwrap_or_default();

        utils::when(
            routing_ref.algorithm_id == Some(algorithm_id.clone()),
            || {
                Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Algorithm is already active".to_string(),
                })
                .into_report()
            },
        )?;

        routing_ref.update_algorithm_id(algorithm_id);
        helpers::update_business_profile_active_algorithm_ref(db, business_profile, routing_ref)
            .await?;

        Ok(service_api::ApplicationResponse::Json(
            routing_algorithm.foreign_into(),
        ))
    }

    #[cfg(not(feature = "business_profile_routing"))]
    {
        let mut routing_ref: routing_types::RoutingAlgorithmRef = merchant_account
            .routing_algorithm
            .clone()
            .map(|val| val.parse_value("RoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to deserialize routing algorithm ref from merchant account")?
            .unwrap_or_default();

        utils::when(
            routing_ref.algorithm_id == Some(algorithm_id.clone()),
            || {
                Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Algorithm is already active".to_string(),
                })
                .into_report()
            },
        )?;
        let mut merchant_dictionary =
            helpers::get_merchant_routing_dictionary(db, &merchant_account.merchant_id).await?;

        let modified_at = common_utils::date_time::now_unix_timestamp();
        let record = merchant_dictionary
            .records
            .iter_mut()
            .find(|rec| rec.id == algorithm_id)
            .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)
            .into_report()
            .attach_printable("Record with given ID not found for routing config activation")?;

        record.modified_at = modified_at;
        merchant_dictionary.active_id = Some(record.id.clone());
        let response = record.clone();
        routing_ref.update_algorithm_id(algorithm_id);
        helpers::update_merchant_routing_dictionary(
            db,
            &merchant_account.merchant_id,
            merchant_dictionary,
        )
        .await?;
        helpers::update_merchant_active_algorithm_ref(db, &key_store, routing_ref).await?;

        Ok(service_api::ApplicationResponse::Json(response))
    }
}

pub async fn retrieve_routing_config(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    algorithm_id: String,
) -> RouterResponse<routing_types::MerchantRoutingAlgorithm> {
    let db = state.store.as_ref();
    #[cfg(feature = "business_profile_routing")]
    {
        let routing_algorithm = db
            .find_routing_algorithm_by_algorithm_id_merchant_id(
                &algorithm_id,
                &merchant_account.merchant_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

        validate_and_get_business_profile(
            &*db.down_cast(),
            Some(&routing_algorithm.profile_id),
            &merchant_account.merchant_id,
        )
        .await?
        .get_required_value("BusinessProfile")
        .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

        let response = routing_algorithm
            .foreign_try_into()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to parse routing algorithm")?;
        Ok(service_api::ApplicationResponse::Json(response))
    }

    #[cfg(not(feature = "business_profile_routing"))]
    {
        let merchant_dictionary =
            helpers::get_merchant_routing_dictionary(db, &merchant_account.merchant_id).await?;

        let record = merchant_dictionary
            .records
            .into_iter()
            .find(|rec| rec.id == algorithm_id)
            .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)
            .into_report()
            .attach_printable("Algorithm with the given ID not found in the merchant dictionary")?;

        let algorithm_config = db
            .find_config_by_key(&algorithm_id)
            .await
            .change_context(errors::ApiErrorResponse::ResourceIdNotFound)
            .attach_printable("Routing config not found in DB")?;

        let algorithm: routing_types::RoutingAlgorithm = algorithm_config
            .config
            .parse_struct("RoutingAlgorithm")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error deserializing routing algorithm config")?;

        let response = routing_types::MerchantRoutingAlgorithm {
            id: record.id,
            name: record.name,
            description: record.description,
            algorithm,
            created_at: record.created_at,
            modified_at: record.modified_at,
        };

        Ok(service_api::ApplicationResponse::Json(response))
    }
}
