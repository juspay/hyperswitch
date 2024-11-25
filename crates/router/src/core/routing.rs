pub mod helpers;
pub mod transformers;
use std::collections::HashSet;

use api_models::{
    enums, mandates as mandates_api, routing,
    routing::{self as routing_types, RoutingRetrieveQuery},
};
use async_trait::async_trait;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use common_utils::ext_traits::AsyncExt;
use diesel_models::routing_algorithm::RoutingAlgorithm;
use error_stack::ResultExt;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use external_services::grpc_client::dynamic_routing::SuccessBasedDynamicRouting;
use hyperswitch_domain_models::{mandates, payment_address};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use router_env::{logger, metrics::add_attributes};
use rustc_hash::FxHashSet;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use storage_impl::redis::cache;

#[cfg(feature = "payouts")]
use super::payouts;
use super::{
    errors::RouterResult,
    payments::{
        routing::{self as payments_routing},
        OperationSessionGetters,
    },
};
#[cfg(feature = "v1")]
use crate::utils::ValueExt;
#[cfg(feature = "v2")]
use crate::{core::admin, utils::ValueExt};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResponse, StorageErrorExt},
        metrics, utils as core_utils,
    },
    db::StorageInterface,
    routes::SessionState,
    services::api as service_api,
    types::{
        api, domain,
        storage::{self, enums as storage_enums},
        transformers::{ForeignInto, ForeignTryFrom},
    },
    utils::{self, OptionExt},
};

pub enum TransactionData<'a> {
    Payment(PaymentsDslInput<'a>),
    #[cfg(feature = "payouts")]
    Payout(&'a payouts::PayoutData),
}

#[derive(Clone)]
pub struct PaymentsDslInput<'a> {
    pub setup_mandate: Option<&'a mandates::MandateData>,
    pub payment_attempt: &'a storage::PaymentAttempt,
    pub payment_intent: &'a storage::PaymentIntent,
    pub payment_method_data: Option<&'a domain::PaymentMethodData>,
    pub address: &'a payment_address::PaymentAddress,
    pub recurring_details: Option<&'a mandates_api::RecurringDetails>,
    pub currency: storage_enums::Currency,
}

impl<'a> PaymentsDslInput<'a> {
    pub fn new(
        setup_mandate: Option<&'a mandates::MandateData>,
        payment_attempt: &'a storage::PaymentAttempt,
        payment_intent: &'a storage::PaymentIntent,
        payment_method_data: Option<&'a domain::PaymentMethodData>,
        address: &'a payment_address::PaymentAddress,
        recurring_details: Option<&'a mandates_api::RecurringDetails>,
        currency: storage_enums::Currency,
    ) -> Self {
        Self {
            setup_mandate,
            payment_attempt,
            payment_intent,
            payment_method_data,
            address,
            recurring_details,
            currency,
        }
    }
}

#[cfg(feature = "v2")]
struct RoutingAlgorithmUpdate(RoutingAlgorithm);

#[cfg(feature = "v2")]
impl RoutingAlgorithmUpdate {
    pub fn create_new_routing_algorithm(
        request: &routing_types::RoutingConfigRequest,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: common_utils::id_type::ProfileId,
        transaction_type: &enums::TransactionType,
    ) -> Self {
        let algorithm_id = common_utils::generate_routing_id_of_default_length();
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
        algorithm_id: &common_utils::id_type::RoutingId,
        db: &dyn StorageInterface,
    ) -> RouterResult<Self> {
        let routing_algo = db
            .find_routing_algorithm_by_algorithm_id_merchant_id(algorithm_id, merchant_id)
            .await
            .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;
        Ok(Self(routing_algo))
    }
}

pub async fn retrieve_merchant_routing_dictionary(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
    query_params: RoutingRetrieveQuery,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingKind> {
    metrics::ROUTING_MERCHANT_DICTIONARY_RETRIEVE.add(&metrics::CONTEXT, 1, &[]);

    let routing_metadata: Vec<diesel_models::routing_algorithm::RoutingProfileMetadata> = state
        .store
        .list_routing_algorithm_metadata_by_merchant_id_transaction_type(
            merchant_account.get_id(),
            transaction_type,
            i64::from(query_params.limit.unwrap_or_default()),
            i64::from(query_params.offset.unwrap_or_default()),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
    let routing_metadata =
        super::utils::filter_objects_based_on_profile_id_list(profile_id_list, routing_metadata);

    let result = routing_metadata
        .into_iter()
        .map(ForeignInto::foreign_into)
        .collect::<Vec<_>>();

    metrics::ROUTING_MERCHANT_DICTIONARY_RETRIEVE_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::RoutingKind::RoutingAlgorithm(result),
    ))
}

#[cfg(feature = "v2")]
pub async fn create_routing_algorithm_under_profile(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    request: routing_types::RoutingConfigRequest,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_CREATE_REQUEST_RECEIVED.add(&metrics::CONTEXT, 1, &[]);
    let db = &*state.store;
    let key_manager_state = &(&state).into();

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&request.profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")?;

    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;

    let all_mcas = helpers::MerchantConnectorAccounts::get_all_mcas(
        merchant_account.get_id(),
        &key_store,
        &state,
    )
    .await?;

    let name_mca_id_set = helpers::ConnectNameAndMCAIdForProfile(
        all_mcas.filter_by_profile(business_profile.get_id(), |mca| {
            (&mca.connector_name, mca.get_id())
        }),
    );

    let name_set = helpers::ConnectNameForProfile(
        all_mcas.filter_by_profile(business_profile.get_id(), |mca| &mca.connector_name),
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
        business_profile.get_id().to_owned(),
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

#[cfg(feature = "v1")]
pub async fn create_routing_algorithm_under_profile(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    request: routing_types::RoutingConfigRequest,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_CREATE_REQUEST_RECEIVED.add(&metrics::CONTEXT, 1, &[]);
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

    let algorithm = request
        .algorithm
        .get_required_value("algorithm")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "algorithm",
        })
        .attach_printable("Algorithm of config not given")?;

    let algorithm_id = common_utils::generate_routing_id_of_default_length();

    let profile_id = request
        .profile_id
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        })
        .attach_printable("Profile_id not provided")?;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")?;

    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;

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

#[cfg(feature = "v2")]
pub async fn link_routing_config_under_profile(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    profile_id: common_utils::id_type::ProfileId,
    algorithm_id: common_utils::id_type::RoutingId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_LINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

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
        key_manager_state,
        &key_store,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")?;

    utils::when(
        routing_algorithm.0.algorithm_for != *transaction_type,
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: format!(
                    "Cannot use {}'s routing algorithm for {} operation",
                    routing_algorithm.0.algorithm_for, transaction_type
                ),
            })
        },
    )?;

    utils::when(
        business_profile.routing_algorithm_id == Some(algorithm_id.clone())
            || business_profile.payout_routing_algorithm_id == Some(algorithm_id.clone()),
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Algorithm is already active".to_string(),
            })
        },
    )?;
    admin::ProfileWrapper::new(business_profile)
        .update_profile_and_invalidate_routing_config_for_active_algorithm_id_update(
            db,
            key_manager_state,
            &key_store,
            algorithm_id,
            transaction_type,
        )
        .await?;

    metrics::ROUTING_LINK_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_algorithm.0.foreign_into(),
    ))
}

#[cfg(feature = "v1")]
pub async fn link_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    algorithm_id: common_utils::id_type::RoutingId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_LINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let routing_algorithm = db
        .find_routing_algorithm_by_algorithm_id_merchant_id(
            &algorithm_id,
            merchant_account.get_id(),
        )
        .await
        .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&routing_algorithm.profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: routing_algorithm.profile_id.get_string_repr().to_owned(),
    })?;

    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;

    match routing_algorithm.kind {
        diesel_models::enums::RoutingAlgorithmKind::Dynamic => {
            let mut dynamic_routing_ref: routing_types::DynamicRoutingAlgorithmRef =
                business_profile
                    .dynamic_routing_algorithm
                    .clone()
                    .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "unable to deserialize Dynamic routing algorithm ref from business profile",
                    )?
                    .unwrap_or_default();

            utils::when(
                matches!(
                    dynamic_routing_ref.success_based_algorithm,
                    Some(routing::SuccessBasedAlgorithm {
                        algorithm_id_with_timestamp:
                        routing_types::DynamicAlgorithmWithTimestamp {
                            algorithm_id: Some(ref id),
                            timestamp: _
                        },
                        enabled_feature: _
                    }) if id == &algorithm_id
                ),
                || {
                    Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: "Algorithm is already active".to_string(),
                    })
                },
            )?;

            dynamic_routing_ref.update_algorithm_id(
                algorithm_id,
                dynamic_routing_ref
                    .success_based_algorithm
                    .clone()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "missing success_based_algorithm in dynamic_algorithm_ref from business_profile table",
                    )?
                    .enabled_feature,
            );
            helpers::update_business_profile_active_dynamic_algorithm_ref(
                db,
                key_manager_state,
                &key_store,
                business_profile,
                dynamic_routing_ref,
            )
            .await?;
        }
        diesel_models::enums::RoutingAlgorithmKind::Single
        | diesel_models::enums::RoutingAlgorithmKind::Priority
        | diesel_models::enums::RoutingAlgorithmKind::Advanced
        | diesel_models::enums::RoutingAlgorithmKind::VolumeSplit => {
            let mut routing_ref: routing_types::RoutingAlgorithmRef = business_profile
                .routing_algorithm
                .clone()
                .map(|val| val.parse_value("RoutingAlgorithmRef"))
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "unable to deserialize routing algorithm ref from business profile",
                )?
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
            helpers::update_profile_active_algorithm_ref(
                db,
                key_manager_state,
                &key_store,
                business_profile,
                routing_ref,
                transaction_type,
            )
            .await?;
        }
    };

    metrics::ROUTING_LINK_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_algorithm.foreign_into(),
    ))
}

#[cfg(feature = "v2")]
pub async fn retrieve_routing_algorithm_from_algorithm_id(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    algorithm_id: common_utils::id_type::RoutingId,
) -> RouterResponse<routing_types::MerchantRoutingAlgorithm> {
    metrics::ROUTING_RETRIEVE_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let routing_algorithm =
        RoutingAlgorithmUpdate::fetch_routing_algo(merchant_account.get_id(), &algorithm_id, db)
            .await?;
    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&routing_algorithm.0.profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;

    let response = routing_types::MerchantRoutingAlgorithm::foreign_try_from(routing_algorithm.0)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse routing algorithm")?;

    metrics::ROUTING_RETRIEVE_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(response))
}

#[cfg(feature = "v1")]
pub async fn retrieve_routing_algorithm_from_algorithm_id(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    algorithm_id: common_utils::id_type::RoutingId,
) -> RouterResponse<routing_types::MerchantRoutingAlgorithm> {
    metrics::ROUTING_RETRIEVE_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let routing_algorithm = db
        .find_routing_algorithm_by_algorithm_id_merchant_id(
            &algorithm_id,
            merchant_account.get_id(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&routing_algorithm.profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;

    let response = routing_types::MerchantRoutingAlgorithm::foreign_try_from(routing_algorithm)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse routing algorithm")?;

    metrics::ROUTING_RETRIEVE_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn unlink_routing_config_under_profile(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    profile_id: common_utils::id_type::ProfileId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UNLINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")?;

    let routing_algo_id = match transaction_type {
        enums::TransactionType::Payment => business_profile.routing_algorithm_id.clone(),
        #[cfg(feature = "payouts")]
        enums::TransactionType::Payout => business_profile.payout_routing_algorithm_id.clone(),
    };

    if let Some(algorithm_id) = routing_algo_id {
        let record = RoutingAlgorithmUpdate::fetch_routing_algo(
            merchant_account.get_id(),
            &algorithm_id,
            db,
        )
        .await?;
        let response = record.0.foreign_into();
        admin::ProfileWrapper::new(business_profile)
            .update_profile_and_invalidate_routing_config_for_active_algorithm_id_update(
                db,
                key_manager_state,
                &key_store,
                algorithm_id,
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

#[cfg(feature = "v1")]
pub async fn unlink_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    request: routing_types::RoutingConfigRequest,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UNLINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);

    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let profile_id = request
        .profile_id
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        })
        .attach_printable("Profile_id not provided")?;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?;

    match business_profile {
        Some(business_profile) => {
            core_utils::validate_profile_id_from_auth_layer(
                authentication_profile_id,
                &business_profile,
            )?;
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
                    helpers::update_profile_active_algorithm_ref(
                        db,
                        key_manager_state,
                        &key_store,
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

#[cfg(feature = "v2")]
pub async fn update_default_fallback_routing(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    profile_id: common_utils::id_type::ProfileId,
    updated_list_of_connectors: Vec<routing_types::RoutableConnectorChoice>,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_UPDATE_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")?;
    let profile_wrapper = admin::ProfileWrapper::new(profile);
    let default_list_of_connectors =
        profile_wrapper.get_default_fallback_list_of_connector_under_profile()?;

    utils::when(
        default_list_of_connectors.len() != updated_list_of_connectors.len(),
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "current config and updated config have different lengths".to_string(),
            })
        },
    )?;

    let existing_set_of_default_connectors: FxHashSet<String> = FxHashSet::from_iter(
        default_list_of_connectors
            .iter()
            .map(|conn_choice| conn_choice.to_string()),
    );
    let updated_set_of_default_connectors: FxHashSet<String> = FxHashSet::from_iter(
        updated_list_of_connectors
            .iter()
            .map(|conn_choice| conn_choice.to_string()),
    );

    let symmetric_diff_between_existing_and_updated_connectors: Vec<String> =
        existing_set_of_default_connectors
            .symmetric_difference(&updated_set_of_default_connectors)
            .cloned()
            .collect();

    utils::when(
        !symmetric_diff_between_existing_and_updated_connectors.is_empty(),
        || {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "connector mismatch between old and new configs ({})",
                    symmetric_diff_between_existing_and_updated_connectors.join(", ")
                ),
            })
        },
    )?;
    profile_wrapper
        .update_default_fallback_routing_of_connectors_under_profile(
            db,
            &updated_list_of_connectors,
            key_manager_state,
            &key_store,
        )
        .await?;

    metrics::ROUTING_UPDATE_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        updated_list_of_connectors,
    ))
}

#[cfg(feature = "v1")]
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

#[cfg(feature = "v2")]
pub async fn retrieve_default_fallback_algorithm_for_profile(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")?;

    let connectors_choice = admin::ProfileWrapper::new(profile)
        .get_default_fallback_list_of_connector_under_profile()?;

    metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(connectors_choice))
}

#[cfg(feature = "v1")]
pub async fn retrieve_default_routing_config(
    state: SessionState,
    profile_id: Option<common_utils::id_type::ProfileId>,
    merchant_account: domain::MerchantAccount,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let id = profile_id
        .map(|profile_id| profile_id.get_string_repr().to_owned())
        .unwrap_or_else(|| merchant_account.get_id().get_string_repr().to_string());

    helpers::get_merchant_default_config(db, &id, transaction_type)
        .await
        .map(|conn_choice| {
            metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG_SUCCESS_RESPONSE.add(
                &metrics::CONTEXT,
                1,
                &[],
            );
            service_api::ApplicationResponse::Json(conn_choice)
        })
}

#[cfg(feature = "v2")]
pub async fn retrieve_routing_config_under_profile(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    query_params: RoutingRetrieveQuery,
    profile_id: common_utils::id_type::ProfileId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::LinkedRoutingConfigRetrieveResponse> {
    metrics::ROUTING_RETRIEVE_LINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        key_manager_state,
        &key_store,
        Some(&profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")?;

    let record = db
        .list_routing_algorithm_metadata_by_profile_id(
            business_profile.get_id(),
            i64::from(query_params.limit.unwrap_or_default()),
            i64::from(query_params.offset.unwrap_or_default()),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let active_algorithms = record
        .into_iter()
        .filter(|routing_rec| &routing_rec.algorithm_for == transaction_type)
        .map(|routing_algo| routing_algo.foreign_into())
        .collect::<Vec<_>>();

    metrics::ROUTING_RETRIEVE_LINK_CONFIG_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::LinkedRoutingConfigRetrieveResponse::ProfileBased(active_algorithms),
    ))
}

#[cfg(feature = "v1")]
pub async fn retrieve_linked_routing_config(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    query_params: routing_types::RoutingRetrieveLinkQuery,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::LinkedRoutingConfigRetrieveResponse> {
    metrics::ROUTING_RETRIEVE_LINK_CONFIG.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let business_profiles = if let Some(profile_id) = query_params.profile_id {
        core_utils::validate_and_get_business_profile(
            db,
            key_manager_state,
            &key_store,
            Some(&profile_id),
            merchant_account.get_id(),
        )
        .await?
        .map(|profile| vec![profile])
        .get_required_value("Profile")
        .change_context(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?
    } else {
        let business_profile = db
            .list_profile_by_merchant_id(key_manager_state, &key_store, merchant_account.get_id())
            .await
            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
        core_utils::filter_objects_based_on_profile_id_list(
            authentication_profile_id.map(|profile_id| vec![profile_id]),
            business_profile.clone(),
        )
    };

    let mut active_algorithms = Vec::new();

    for business_profile in business_profiles {
        let profile_id = business_profile.get_id().to_owned();

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
                    &profile_id,
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
// List all the default fallback algorithms under all the profile under a merchant
pub async fn retrieve_default_routing_config_for_profiles(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::ProfileDefaultRoutingConfig>> {
    metrics::ROUTING_RETRIEVE_CONFIG_FOR_PROFILE.add(&metrics::CONTEXT, 1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let all_profiles = db
        .list_profile_by_merchant_id(key_manager_state, &key_store, merchant_account.get_id())
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)
        .attach_printable("error retrieving all business profiles for merchant")?;

    let retrieve_config_futures = all_profiles
        .iter()
        .map(|prof| {
            helpers::get_merchant_default_config(
                db,
                prof.get_id().get_string_repr(),
                transaction_type,
            )
        })
        .collect::<Vec<_>>();

    let configs = futures::future::join_all(retrieve_config_futures)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    let default_configs = configs
        .into_iter()
        .zip(all_profiles.iter().map(|prof| prof.get_id().to_owned()))
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
    key_store: domain::MerchantKeyStore,
    updated_config: Vec<routing_types::RoutableConnectorChoice>,
    profile_id: common_utils::id_type::ProfileId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::ProfileDefaultRoutingConfig> {
    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE.add(&metrics::CONTEXT, 1, &[]);

    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let business_profile = core_utils::validate_and_get_business_profile(
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
    let default_config = helpers::get_merchant_default_config(
        db,
        business_profile.get_id().get_string_repr(),
        transaction_type,
    )
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
        business_profile.get_id().get_string_repr(),
        updated_config.clone(),
        transaction_type,
    )
    .await?;

    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE_SUCCESS_RESPONSE.add(&metrics::CONTEXT, 1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::ProfileDefaultRoutingConfig {
            profile_id: business_profile.get_id().to_owned(),
            connectors: updated_config,
        },
    ))
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
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
                            dynamic_routing_volume_split: u8::default(),
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

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
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

    let prefix_of_dynamic_routing_keys = helpers::generate_tenant_business_profile_id(
        &state.tenant.redis_key_prefix,
        profile_id.get_string_repr(),
    );
    state
        .grpc_client
        .dynamic_routing
        .success_rate_client
        .as_ref()
        .async_map(|sr_client| async {
            sr_client
                .invalidate_success_rate_routing_keys(prefix_of_dynamic_routing_keys)
                .await
                .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "Failed to invalidate the routing keys".to_string(),
                })
        })
        .await
        .transpose()?;

    Ok(service_api::ApplicationResponse::Json(new_record))
}

#[async_trait]
pub trait GetRoutableConnectorsForChoice {
    async fn get_routable_connectors(
        &self,
        db: &dyn StorageInterface,
        business_profile: &domain::Profile,
    ) -> RouterResult<RoutableConnectors>;
}

pub struct StraightThroughAlgorithmTypeSingle(pub serde_json::Value);

#[async_trait]
impl GetRoutableConnectorsForChoice for StraightThroughAlgorithmTypeSingle {
    async fn get_routable_connectors(
        &self,
        _db: &dyn StorageInterface,
        _business_profile: &domain::Profile,
    ) -> RouterResult<RoutableConnectors> {
        let straight_through_routing_algorithm = self
            .0
            .clone()
            .parse_value::<api::routing::StraightThroughAlgorithm>("RoutingAlgorithm")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse the straight through routing algorithm")?;
        let routable_connector = match straight_through_routing_algorithm {
            api::routing::StraightThroughAlgorithm::Single(connector) => {
                vec![*connector]
            }

            api::routing::StraightThroughAlgorithm::Priority(_)
            | api::routing::StraightThroughAlgorithm::VolumeSplit(_) => {
                Err(errors::RoutingError::DslIncorrectSelectionAlgorithm)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Unsupported algorithm received as a result of static routing",
                    )?
            }
        };
        Ok(RoutableConnectors(routable_connector))
    }
}

pub struct DecideConnector;

#[async_trait]
impl GetRoutableConnectorsForChoice for DecideConnector {
    async fn get_routable_connectors(
        &self,
        db: &dyn StorageInterface,
        business_profile: &domain::Profile,
    ) -> RouterResult<RoutableConnectors> {
        let fallback_config = helpers::get_merchant_default_config(
            db,
            business_profile.get_id().get_string_repr(),
            &common_enums::TransactionType::Payment,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
        Ok(RoutableConnectors(fallback_config))
    }
}

pub struct RoutableConnectors(Vec<routing_types::RoutableConnectorChoice>);

impl RoutableConnectors {
    pub fn filter_network_transaction_id_flow_supported_connectors(
        self,
        nit_connectors: HashSet<String>,
    ) -> Self {
        let connectors = self
            .0
            .into_iter()
            .filter(|routable_connector_choice| {
                nit_connectors.contains(&routable_connector_choice.connector.to_string())
            })
            .collect();
        Self(connectors)
    }

    pub async fn construct_dsl_and_perform_eligibility_analysis<F, D>(
        self,
        state: &SessionState,
        key_store: &domain::MerchantKeyStore,
        payment_data: &D,

        profile_id: &common_utils::id_type::ProfileId,
    ) -> RouterResult<Vec<api::ConnectorData>>
    where
        F: Send + Clone,
        D: OperationSessionGetters<F>,
    {
        let payments_dsl_input = PaymentsDslInput::new(
            payment_data.get_setup_mandate(),
            payment_data.get_payment_attempt(),
            payment_data.get_payment_intent(),
            payment_data.get_payment_method_data(),
            payment_data.get_address(),
            payment_data.get_recurring_details(),
            payment_data.get_currency(),
        );

        let routable_connector_choice = self.0.clone();

        let backend_input = payments_routing::make_dsl_input(&payments_dsl_input)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to construct dsl input")?;

        let connectors = payments_routing::perform_cgraph_filtering(
            state,
            key_store,
            routable_connector_choice,
            backend_input,
            None,
            profile_id,
            &common_enums::TransactionType::Payment,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Eligibility analysis failed for routable connectors")?;

        let connector_data = connectors
            .into_iter()
            .map(|conn| {
                api::ConnectorData::get_connector_by_name(
                    &state.conf.connectors,
                    &conn.connector.to_string(),
                    api::GetToken::Connector,
                    conn.merchant_connector_id.clone(),
                )
            })
            .collect::<CustomResult<Vec<_>, _>>()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid connector name received")?;

        Ok(connector_data)
    }
}
