pub mod helpers;
pub mod transformers;

use api_models::routing as routing_types;
#[cfg(feature = "business_profile_routing")]
use api_models::routing::{RoutingRetrieveLinkQuery, RoutingRetrieveQuery};
#[cfg(not(feature = "business_profile_routing"))]
use common_utils::ext_traits::{Encode, StringExt};
#[cfg(not(feature = "business_profile_routing"))]
use diesel_models::configs;
#[cfg(feature = "business_profile_routing")]
use diesel_models::routing_algorithm::RoutingAlgorithm;
use error_stack::{IntoReport, ResultExt};
use rustc_hash::FxHashSet;

#[cfg(feature = "business_profile_routing")]
use crate::core::utils::validate_and_get_business_profile;
#[cfg(feature = "business_profile_routing")]
use crate::types::transformers::{ForeignInto, ForeignTryInto};
use crate::{
    consts,
    core::errors::{RouterResponse, StorageErrorExt},
    routes::AppState,
    types::domain,
    utils::{self, OptionExt, ValueExt},
};
#[cfg(not(feature = "business_profile_routing"))]
use crate::{core::errors, services::api as service_api, types::storage};
#[cfg(feature = "business_profile_routing")]
use crate::{errors, services::api as service_api};

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
    key_store: domain::MerchantKeyStore,
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

        validate_and_get_business_profile(db, Some(&profile_id), &merchant_account.merchant_id)
            .await?;

        helpers::validate_connectors_in_routing_config(
            db,
            &key_store,
            &merchant_account.merchant_id,
            &profile_id,
            &algorithm,
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
            db,
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
            db,
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
pub async fn unlink_routing_config(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    #[cfg(not(feature = "business_profile_routing"))] key_store: domain::MerchantKeyStore,
    #[cfg(feature = "business_profile_routing")] request: routing_types::RoutingConfigRequest,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    let db = state.store.as_ref();
    #[cfg(feature = "business_profile_routing")]
    {
        let profile_id = request
            .profile_id
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "profile_id",
            })
            .attach_printable("Profile_id not provided")?;
        let business_profile =
            validate_and_get_business_profile(db, Some(&profile_id), &merchant_account.merchant_id)
                .await?;
        match business_profile {
            Some(business_profile) => {
                let routing_algo_ref: routing_types::RoutingAlgorithmRef = business_profile
                    .routing_algorithm
                    .clone()
                    .map(|val| val.parse_value("RoutingAlgorithmRef"))
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "unable to deserialize routing algorithm ref from merchant account",
                    )?
                    .unwrap_or_default();

                let timestamp = common_utils::date_time::now_unix_timestamp();

                match routing_algo_ref.algorithm_id {
                    Some(algorithm_id) => {
                        let routing_algorithm: routing_types::RoutingAlgorithmRef =
                            routing_types::RoutingAlgorithmRef {
                                algorithm_id: None,
                                timestamp,
                                config_algo_id: routing_algo_ref.config_algo_id.clone(),
                                surcharge_config_algo_id: routing_algo_ref.surcharge_config_algo_id,
                            };

                        let record = db
                            .find_routing_algorithm_by_profile_id_algorithm_id(
                                &profile_id,
                                &algorithm_id,
                            )
                            .await
                            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
                        let response = record.foreign_into();
                        helpers::update_business_profile_active_algorithm_ref(
                            db,
                            business_profile,
                            routing_algorithm,
                        )
                        .await?;
                        Ok(service_api::ApplicationResponse::Json(response))
                    }
                    None => Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Algorithm is already inactive".to_string(),
                    })
                    .into_report()?,
                }
            }
            None => Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "The business_profile is not present".to_string(),
            }
            .into()),
        }
    }

    #[cfg(not(feature = "business_profile_routing"))]
    {
        let mut merchant_dictionary =
            helpers::get_merchant_routing_dictionary(db, &merchant_account.merchant_id).await?;

        let routing_algo_ref: routing_types::RoutingAlgorithmRef = merchant_account
            .routing_algorithm
            .clone()
            .map(|val| val.parse_value("RoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to deserialize routing algorithm ref from merchant account")?
            .unwrap_or_default();
        let timestamp = common_utils::date_time::now_unix_timestamp();

        utils::when(routing_algo_ref.algorithm_id.is_none(), || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Algorithm is already inactive".to_string(),
            })
            .into_report()
        })?;
        let routing_algorithm: routing_types::RoutingAlgorithmRef =
            routing_types::RoutingAlgorithmRef {
                algorithm_id: None,
                timestamp,
                config_algo_id: routing_algo_ref.config_algo_id.clone(),
                surcharge_config_algo_id: routing_algo_ref.surcharge_config_algo_id,
            };

        let active_algorithm_id = merchant_dictionary
            .active_id
            .or(routing_algo_ref.algorithm_id.clone())
            .ok_or(errors::ApiErrorResponse::PreconditionFailed {
                // When the merchant_dictionary doesn't have any active algorithm and merchant_account doesn't have any routing_algorithm configured
                message: "Algorithm is already inactive".to_string(),
            })
            .into_report()?;

        let record = merchant_dictionary
            .records
            .iter_mut()
            .find(|rec| rec.id == active_algorithm_id)
            .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)
            .into_report()
            .attach_printable("Record with the given ID not found for de-activation")?;

        let response = record.clone();

        merchant_dictionary.active_id = None;

        helpers::update_merchant_routing_dictionary(
            db,
            &merchant_account.merchant_id,
            merchant_dictionary,
        )
        .await?;

        let ref_value =
            Encode::<routing_types::RoutingAlgorithmRef>::encode_to_value(&routing_algorithm)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed converting routing algorithm ref to json value")?;

        let merchant_account_update = storage::MerchantAccountUpdate::Update {
            merchant_name: None,
            merchant_details: None,
            return_url: None,
            webhook_details: None,
            sub_merchants_enabled: None,
            parent_merchant_id: None,
            enable_payment_response_hash: None,
            payment_response_hash_key: None,
            redirect_to_merchant_with_http_post: None,
            publishable_key: None,
            locker_id: None,
            metadata: None,
            routing_algorithm: Some(ref_value),
            primary_business_details: None,
            intent_fulfillment_time: None,
            frm_routing_algorithm: None,
            payout_routing_algorithm: None,
            default_profile: None,
            payment_link_config: None,
        };

        db.update_specific_fields_in_merchant(
            &key_store.merchant_id,
            merchant_account_update,
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update routing algorithm ref in merchant account")?;

        Ok(service_api::ApplicationResponse::Json(response))
    }
}

pub async fn update_default_routing_config(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    updated_config: Vec<routing_types::RoutableConnectorChoice>,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    let db = state.store.as_ref();
    let default_config =
        helpers::get_merchant_default_config(db, &merchant_account.merchant_id).await?;

    utils::when(default_config.len() != updated_config.len(), || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "current config and updated config have different lengths".to_string(),
        })
        .into_report()
    })?;

    let existing_set: FxHashSet<String> =
        FxHashSet::from_iter(default_config.iter().map(|c| c.to_string()));
    let updated_set: FxHashSet<String> =
        FxHashSet::from_iter(updated_config.iter().map(|c| c.to_string()));

    let symmetric_diff: Vec<String> = existing_set
        .symmetric_difference(&updated_set)
        .cloned()
        .collect();

    utils::when(!symmetric_diff.is_empty(), || {
        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "connector mismatch between old and new configs ({})",
                symmetric_diff.join(", ")
            ),
        })
        .into_report()
    })?;

    helpers::update_merchant_default_config(
        db,
        &merchant_account.merchant_id,
        updated_config.clone(),
    )
    .await?;

    Ok(service_api::ApplicationResponse::Json(updated_config))
}

pub async fn retrieve_default_routing_config(
    state: AppState,
    merchant_account: domain::MerchantAccount,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    let db = state.store.as_ref();

    helpers::get_merchant_default_config(db, &merchant_account.merchant_id)
        .await
        .map(service_api::ApplicationResponse::Json)
}

pub async fn retrieve_linked_routing_config(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    #[cfg(feature = "business_profile_routing")] query_params: RoutingRetrieveLinkQuery,
) -> RouterResponse<routing_types::LinkedRoutingConfigRetrieveResponse> {
    let db = state.store.as_ref();

    #[cfg(feature = "business_profile_routing")]
    {
        let business_profiles = if let Some(profile_id) = query_params.profile_id {
            validate_and_get_business_profile(db, Some(&profile_id), &merchant_account.merchant_id)
                .await?
                .map(|profile| vec![profile])
                .get_required_value("BusinessProfile")
                .change_context(errors::ApiErrorResponse::BusinessProfileNotFound {
                    id: profile_id,
                })?
        } else {
            db.list_business_profile_by_merchant_id(&merchant_account.merchant_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?
        };

        let mut active_algorithms = Vec::new();

        for business_profile in business_profiles {
            let routing_ref: routing_types::RoutingAlgorithmRef = business_profile
                .routing_algorithm
                .clone()
                .map(|val| val.parse_value("RoutingAlgorithmRef"))
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "unable to deserialize routing algorithm ref from merchant account",
                )?
                .unwrap_or_default();

            if let Some(algorithm_id) = routing_ref.algorithm_id {
                let record = db
                    .find_routing_algorithm_metadata_by_algorithm_id_profile_id(
                        &algorithm_id,
                        &business_profile.profile_id,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

                active_algorithms.push(record.foreign_into());
            }
        }

        Ok(service_api::ApplicationResponse::Json(
            routing_types::LinkedRoutingConfigRetrieveResponse::ProfileBased(active_algorithms),
        ))
    }
    #[cfg(not(feature = "business_profile_routing"))]
    {
        let merchant_dictionary =
            helpers::get_merchant_routing_dictionary(db, &merchant_account.merchant_id).await?;

        let algorithm = if let Some(algorithm_id) = merchant_dictionary.active_id {
            let record = merchant_dictionary
                .records
                .into_iter()
                .find(|rec| rec.id == algorithm_id)
                .ok_or(errors::ApiErrorResponse::ResourceIdNotFound)
                .into_report()
                .attach_printable("record for active algorithm not found in merchant dictionary")?;

            let config = db
                .find_config_by_key(&algorithm_id)
                .await
                .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("error finding routing config in db")?;

            let the_algorithm: routing_types::RoutingAlgorithm = config
                .config
                .parse_struct("RoutingAlgorithm")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("unable to parse routing algorithm")?;

            Some(routing_types::MerchantRoutingAlgorithm {
                id: record.id,
                name: record.name,
                description: record.description,
                algorithm: the_algorithm,
                created_at: record.created_at,
                modified_at: record.modified_at,
            })
        } else {
            None
        };

        let response = routing_types::LinkedRoutingConfigRetrieveResponse::MerchantAccountBased(
            routing_types::RoutingRetrieveResponse { algorithm },
        );

        Ok(service_api::ApplicationResponse::Json(response))
    }
}
