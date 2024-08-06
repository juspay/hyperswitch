pub mod helpers;
pub mod transformers;

#[cfg(all(feature = "v2", feature = "routing_v2"))]
use api_models::routing::RoutingConfigRequest;
use api_models::{
    enums,
    routing::{self as routing_types, RoutingRetrieveLinkQuery, RoutingRetrieveQuery},
};
use diesel_models::routing_algorithm::RoutingAlgorithm;
use error_stack::ResultExt;
#[cfg(all(feature = "v2", feature = "routing_v2"))]
use masking::Secret;
use rustc_hash::FxHashSet;

use super::payments;
#[cfg(feature = "payouts")]
use super::payouts;
use crate::{
    consts,
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        metrics, utils as core_utils,
    },
    routes::SessionState,
    services::api as service_api,
    types::{
        domain,
        transformers::{ForeignInto, ForeignTryFrom},
    },
    utils::{self, OptionExt, ValueExt},
};
#[cfg(all(feature = "v2", feature = "routing_v2"))]
use crate::{core::errors::RouterResult, db::StorageInterface};
pub enum TransactionData<'a, F>
where
    F: Clone,
{
    Payment(&'a mut payments::PaymentData<F>),
    #[cfg(feature = "payouts")]
    Payout(&'a payouts::PayoutData),
}

#[cfg(all(feature = "v2", feature = "routing_v2"))]
struct RoutingAlgorithmUpdate(RoutingAlgorithm);

#[cfg(all(feature = "v2", feature = "routing_v2"))]
impl RoutingAlgorithmUpdate {
    pub fn create_new_routing_algorithm(
        request: &RoutingConfigRequest,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: String,
        transaction_type: &enums::TransactionType,
    ) -> Self {
        let algorithm_id = common_utils::generate_id(
            consts::ROUTING_CONFIG_ID_LENGTH,
            &format!("routing_{}", merchant_id.get_string_repr()),
        );
        let timestamp = common_utils::date_time::now();
        let algo = RoutingAlgorithm {
            algorithm_id,
            profile_id,
            merchant_id: merchant_id.clone(),
            name: request.name.clone(),
            description: Some(request.description.clone()),
            kind: request.algorithm.get_kind().foreign_into(),
            algorithm_data: serde_json::json!(request.algorithm),
            created_at: timestamp,
            modified_at: timestamp,
            algorithm_for: transaction_type.to_owned(),
        };
        Self(algo)
    }
    pub async fn fetch_routing_algo(
        merchant_id: &common_utils::id_type::MerchantId,
        algorithm_id: &str,
        db: &dyn StorageInterface,
    ) -> RouterResult<Self> {
        let routing_algo = db
            .find_routing_algorithm_by_algorithm_id_merchant_id(algorithm_id, merchant_id)
            .await
            .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;
        Ok(Self(routing_algo))
    }

    pub fn update_routing_ref_with_algorithm_id(
        &self,
        transaction_type: &enums::TransactionType,
        routing_ref: &mut routing_types::RoutingAlgorithmRef,
    ) -> RouterResult<()> {
        utils::when(self.0.algorithm_for != *transaction_type, || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: format!(
                    "Cannot use {}'s routing algorithm for {} operation",
                    self.0.algorithm_for, transaction_type
                ),
            })
        })?;

        utils::when(
            routing_ref.algorithm_id == Some(self.0.algorithm_id.clone()),
            || {
                Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Algorithm is already active".to_string(),
                })
            },
        )?;
        routing_ref.update_algorithm_id(self.0.algorithm_id.clone());
        Ok(())
    }
}

pub async fn retrieve_merchant_routing_dictionary(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    query_params: RoutingRetrieveQuery,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingKind> {
    metrics::ROUTING_MERCHANT_DICTIONARY_RETRIEVE.add(&metrics::CONTEXT, 1, &[]);

    let routing_metadata = state
        .store
        .list_routing_algorithm_metadata_by_merchant_id_transaction_type(
            merchant_account.get_id(),
            transaction_type,
            i64::from(query_params.limit.unwrap_or_default()),
            i64::from(query_params.offset.unwrap_or_default()),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
    let result = routing_metadata
        .into_iter()
        .map(ForeignInto::foreign_into)
        .collect::<Vec<_>>();

    metrics::ROUTING_MERCHANT_DICTIONARY_RETRIEVE_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::RoutingKind::RoutingAlgorithm(result),
    ))
}

#[cfg(all(feature = "v2", feature = "routing_v2"))]
pub async fn create_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    request: RoutingConfigRequest,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_CREATE_REQUEST_RECEIVED.add(&metrics::CONTEXT, 1, &[]);
    let db = &*state.store;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        Some(&request.profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("BusinessProfile")?;

    let all_mcas = helpers::MerchantConnectorAccounts::get_all_mcas(
        merchant_account.get_id(),
        &key_store,
        &state,
    )
    .await?;

    let name_mca_id_set = helpers::ConnectNameAndMCAIdForProfile(
        all_mcas.filter_by_profile(&business_profile.profile_id, |mca| {
            (&mca.connector_name, &mca.merchant_connector_id)
        }),
    );

    let name_set = helpers::ConnectNameForProfile(
        all_mcas.filter_by_profile(&business_profile.profile_id, |mca| &mca.connector_name),
    );

    let algorithm_helper = helpers::RoutingAlgorithmHelpers {
        name_mca_id_set,
        name_set,
        routing_algorithm: &request.algorithm,
    };

    algorithm_helper.validate_connectors_in_routing_config()?;

    let algo = RoutingAlgorithmUpdate::create_new_routing_algorithm(
        &request,
        merchant_account.get_id(),
        business_profile.profile_id,
        transaction_type,
    );

    let record = state
        .store
        .as_ref()
        .insert_routing_algorithm(algo.0)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let new_record = record.foreign_into();

    metrics::ROUTING_CREATE_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(new_record))
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "routing_v2")))]
pub async fn create_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    request: routing_types::RoutingConfigRequest,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_CREATE_REQUEST_RECEIVED.add(&metrics::CONTEXT, 1, &[]);
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
        &format!("routing_{}", merchant_account.get_id().get_string_repr()),
    );

    let profile_id = request
        .profile_id
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        })
        .attach_printable("Profile_id not provided")?;

    core_utils::validate_and_get_business_profile(db, Some(&profile_id), merchant_account.get_id())
        .await?;

    helpers::validate_connectors_in_routing_config(
        &state,
        &key_store,
        merchant_account.get_id(),
        &profile_id,
        &algorithm,
    )
    .await?;

    let timestamp = common_utils::date_time::now();
    let algo = RoutingAlgorithm {
        algorithm_id: algorithm_id.clone(),
        profile_id,
        merchant_id: merchant_account.get_id().to_owned(),
        name: name.clone(),
        description: Some(description.clone()),
        kind: algorithm.get_kind().foreign_into(),
        algorithm_data: serde_json::json!(algorithm),
        created_at: timestamp,
        modified_at: timestamp,
        algorithm_for: transaction_type.to_owned(),
    };
    let record = db
        .insert_routing_algorithm(algo)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let new_record = record.foreign_into();

    metrics::ROUTING_CREATE_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(new_record))
}

#[cfg(all(feature = "v2", feature = "routing_v2"))]
pub async fn link_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile_id: String,
    algorithm_id: String,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_LINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();

    let routing_algorithm =
        RoutingAlgorithmUpdate::fetch_routing_algo(merchant_account.get_id(), &algorithm_id, db)
            .await?;

    utils::when(routing_algorithm.0.profile_id != profile_id, || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "Profile Id is invalid for the routing config".to_string(),
        })
    })?;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("BusinessProfile")?;

    let mut routing_ref = routing_types::RoutingAlgorithmRef::parse_routing_algorithm(
        business_profile.routing_algorithm.clone().map(Secret::new),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("unable to deserialize routing algorithm ref from merchant account")?
    .unwrap_or_default();

    routing_algorithm.update_routing_ref_with_algorithm_id(transaction_type, &mut routing_ref)?;

    // TODO move to business profile
    helpers::update_business_profile_active_algorithm_ref(
        db,
        business_profile,
        routing_ref,
        transaction_type,
    )
    .await?;
    metrics::ROUTING_LINK_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_algorithm.0.foreign_into(),
    ))
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "routing_v2")))]
pub async fn link_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    algorithm_id: String,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_LINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();

    let routing_algorithm = db
        .find_routing_algorithm_by_algorithm_id_merchant_id(
            &algorithm_id,
            merchant_account.get_id(),
        )
        .await
        .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        Some(&routing_algorithm.profile_id),
        merchant_account.get_id(),
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

    utils::when(routing_algorithm.algorithm_for != *transaction_type, || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "Cannot use {}'s routing algorithm for {} operation",
                routing_algorithm.algorithm_for, transaction_type
            ),
        })
    })?;

    utils::when(
        routing_ref.algorithm_id == Some(algorithm_id.clone()),
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Algorithm is already active".to_string(),
            })
        },
    )?;
    routing_ref.update_algorithm_id(algorithm_id);
    helpers::update_business_profile_active_algorithm_ref(
        db,
        business_profile,
        routing_ref,
        transaction_type,
    )
    .await?;

    metrics::ROUTING_LINK_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_algorithm.foreign_into(),
    ))
}

#[cfg(all(feature = "v2", feature = "routing_v2"))]
pub async fn retrieve_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    algorithm_id: routing_types::RoutingAlgorithmId,
) -> RouterResponse<routing_types::MerchantRoutingAlgorithm> {
    metrics::ROUTING_RETRIEVE_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();

    let routing_algorithm =
        RoutingAlgorithmUpdate::fetch_routing_algo(merchant_account.get_id(), &algorithm_id.0, db)
            .await?;
    // TODO: Move to domain types of Business Profile
    core_utils::validate_and_get_business_profile(
        db,
        Some(&routing_algorithm.0.profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("BusinessProfile")
    .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let response = routing_types::MerchantRoutingAlgorithm::foreign_try_from(routing_algorithm.0)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse routing algorithm")?;

    metrics::ROUTING_RETRIEVE_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(response))
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "routing_v2")))]
pub async fn retrieve_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    algorithm_id: routing_types::RoutingAlgorithmId,
) -> RouterResponse<routing_types::MerchantRoutingAlgorithm> {
    metrics::ROUTING_RETRIEVE_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();

    let routing_algorithm = db
        .find_routing_algorithm_by_algorithm_id_merchant_id(
            &algorithm_id.0,
            merchant_account.get_id(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    core_utils::validate_and_get_business_profile(
        db,
        Some(&routing_algorithm.profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("BusinessProfile")
    .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let response = routing_types::MerchantRoutingAlgorithm::foreign_try_from(routing_algorithm)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse routing algorithm")?;

    metrics::ROUTING_RETRIEVE_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(response))
}

#[cfg(all(feature = "v2", feature = "routing_v2"))]
pub async fn unlink_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile_id: String,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UNLINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("BusinessProfile")?;

    let routing_algo_ref = routing_types::RoutingAlgorithmRef::parse_routing_algorithm(
        match transaction_type {
            enums::TransactionType::Payment => business_profile.routing_algorithm.clone(),
            #[cfg(feature = "payouts")]
            enums::TransactionType::Payout => business_profile.payout_routing_algorithm.clone(),
        }
        .map(Secret::new),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("unable to deserialize routing algorithm ref from merchant account")?
    .unwrap_or_default();

    if let Some(algorithm_id) = routing_algo_ref.algorithm_id {
        let timestamp = common_utils::date_time::now_unix_timestamp();
        let routing_algorithm: routing_types::RoutingAlgorithmRef =
            routing_types::RoutingAlgorithmRef {
                algorithm_id: None,
                timestamp,
                config_algo_id: routing_algo_ref.config_algo_id.clone(),
                surcharge_config_algo_id: routing_algo_ref.surcharge_config_algo_id,
            };
        let record = RoutingAlgorithmUpdate::fetch_routing_algo(
            merchant_account.get_id(),
            &algorithm_id,
            db,
        )
        .await?;
        let response = record.0.foreign_into();
        helpers::update_business_profile_active_algorithm_ref(
            db,
            business_profile,
            routing_algorithm,
            transaction_type,
        )
        .await?;

        metrics::ROUTING_UNLINK_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
        Ok(service_api::ApplicationResponse::Json(response))
    } else {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "Algorithm is already inactive".to_string(),
        })?
    }
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "routing_v2")))]
pub async fn unlink_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    request: routing_types::RoutingConfigRequest,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UNLINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let profile_id = request
        .profile_id
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        })
        .attach_printable("Profile_id not provided")?;
    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?;
    match business_profile {
        Some(business_profile) => {
            let routing_algo_ref: routing_types::RoutingAlgorithmRef = match transaction_type {
                enums::TransactionType::Payment => business_profile.routing_algorithm.clone(),
                #[cfg(feature = "payouts")]
                enums::TransactionType::Payout => business_profile.payout_routing_algorithm.clone(),
            }
            .map(|val| val.parse_value("RoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to deserialize routing algorithm ref from merchant account")?
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
                        transaction_type,
                    )
                    .await?;

                    metrics::ROUTING_UNLINK_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
                    Ok(service_api::ApplicationResponse::Json(response))
                }
                None => Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Algorithm is already inactive".to_string(),
                })?,
            }
        }
        None => Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "The business_profile is not present".to_string(),
        }
        .into()),
    }
}

//feature update
pub async fn update_default_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    updated_config: Vec<routing_types::RoutableConnectorChoice>,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_UPDATE_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let default_config = helpers::get_merchant_default_config(
        db,
        merchant_account.get_id().get_string_repr(),
        transaction_type,
    )
    .await?;

    utils::when(default_config.len() != updated_config.len(), || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "current config and updated config have different lengths".to_string(),
        })
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
    })?;

    helpers::update_merchant_default_config(
        db,
        merchant_account.get_id().get_string_repr(),
        updated_config.clone(),
        transaction_type,
    )
    .await?;

    metrics::ROUTING_UPDATE_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(updated_config))
}

pub async fn retrieve_default_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();

    helpers::get_merchant_default_config(
        db,
        merchant_account.get_id().get_string_repr(),
        transaction_type,
    )
    .await
    .map(|conn_choice| {
        metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
        service_api::ApplicationResponse::Json(conn_choice)
    })
}

pub async fn retrieve_linked_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    query_params: RoutingRetrieveLinkQuery,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::LinkedRoutingConfigRetrieveResponse> {
    metrics::ROUTING_RETRIEVE_LINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();

    let business_profiles = if let Some(profile_id) = query_params.profile_id {
        core_utils::validate_and_get_business_profile(
            db,
            Some(&profile_id),
            merchant_account.get_id(),
        )
        .await?
        .map(|profile| vec![profile])
        .get_required_value("BusinessProfile")
        .change_context(errors::ApiErrorResponse::BusinessProfileNotFound { id: profile_id })?
    } else {
        db.list_business_profile_by_merchant_id(merchant_account.get_id())
            .await
            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?
    };

    let mut active_algorithms = Vec::new();

    for business_profile in business_profiles {
        let routing_ref: routing_types::RoutingAlgorithmRef = match transaction_type {
            enums::TransactionType::Payment => business_profile.routing_algorithm,
            #[cfg(feature = "payouts")]
            enums::TransactionType::Payout => business_profile.payout_routing_algorithm,
        }
        .clone()
        .map(|val| val.parse_value("RoutingAlgorithmRef"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to deserialize routing algorithm ref from merchant account")?
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

    metrics::ROUTING_RETRIEVE_LINK_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::LinkedRoutingConfigRetrieveResponse::ProfileBased(active_algorithms),
    ))
}

pub async fn retrieve_default_routing_config_for_profiles(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::ProfileDefaultRoutingConfig>> {
    metrics::ROUTING_RETRIEVE_CONFIG_FOR_PROFILE.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();

    let all_profiles = db
        .list_business_profile_by_merchant_id(merchant_account.get_id())
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)
        .attach_printable("error retrieving all business profiles for merchant")?;

    let retrieve_config_futures = all_profiles
        .iter()
        .map(|prof| helpers::get_merchant_default_config(db, &prof.profile_id, transaction_type))
        .collect::<Vec<_>>();

    let configs = futures::future::join_all(retrieve_config_futures)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    let default_configs = configs
        .into_iter()
        .zip(all_profiles.iter().map(|prof| prof.profile_id.clone()))
        .map(
            |(config, profile_id)| routing_types::ProfileDefaultRoutingConfig {
                profile_id,
                connectors: config,
            },
        )
        .collect::<Vec<_>>();

    metrics::ROUTING_RETRIEVE_CONFIG_FOR_PROFILE_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(default_configs))
}

pub async fn update_default_routing_config_for_profile(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    updated_config: Vec<routing_types::RoutableConnectorChoice>,
    profile_id: String,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::ProfileDefaultRoutingConfig> {
    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("BusinessProfile")
    .change_context(errors::ApiErrorResponse::BusinessProfileNotFound { id: profile_id })?;
    let default_config =
        helpers::get_merchant_default_config(db, &business_profile.profile_id, transaction_type)
            .await?;

    utils::when(default_config.len() != updated_config.len(), || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "current config and updated config have different lengths".to_string(),
        })
    })?;

    let existing_set = FxHashSet::from_iter(
        default_config
            .iter()
            .map(|c| (c.connector.to_string(), c.merchant_connector_id.as_ref())),
    );

    let updated_set = FxHashSet::from_iter(
        updated_config
            .iter()
            .map(|c| (c.connector.to_string(), c.merchant_connector_id.as_ref())),
    );

    let symmetric_diff = existing_set
        .symmetric_difference(&updated_set)
        .cloned()
        .collect::<Vec<_>>();

    utils::when(!symmetric_diff.is_empty(), || {
        let error_str = symmetric_diff
            .into_iter()
            .map(|(connector, ident)| format!("'{connector}:{ident:?}'"))
            .collect::<Vec<_>>()
            .join(", ");

        Err(errors::ApiErrorResponse::InvalidRequestData {
            message: format!("connector mismatch between old and new configs ({error_str})"),
        })
    })?;

    helpers::update_merchant_default_config(
        db,
        &business_profile.profile_id,
        updated_config.clone(),
        transaction_type,
    )
    .await?;

    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::ProfileDefaultRoutingConfig {
            profile_id: business_profile.profile_id,
            connectors: updated_config,
        },
    ))
}
