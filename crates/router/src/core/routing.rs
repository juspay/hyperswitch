pub mod helpers;
pub mod transformers;
use std::collections::HashSet;

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::routing::DynamicRoutingAlgoAccessor;
use api_models::{
    enums, mandates as mandates_api,
    open_router::{
        DecideGatewayResponse, OpenRouterDecideGatewayRequest, UpdateScorePayload,
        UpdateScoreResponse,
    },
    routing,
    routing::{
        self as routing_types, RoutingRetrieveQuery, RuleMigrationError, RuleMigrationResponse,
    },
};
use async_trait::async_trait;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use common_utils::ext_traits::AsyncExt;
use common_utils::request::Method;
use diesel_models::routing_algorithm::RoutingAlgorithm;
use error_stack::ResultExt;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use external_services::grpc_client::dynamic_routing::{
    contract_routing_client::ContractBasedDynamicRouting,
    elimination_based_client::EliminationBasedRouting,
    success_rate_client::SuccessBasedDynamicRouting,
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use helpers::{
    enable_decision_engine_dynamic_routing_setup, update_decision_engine_dynamic_routing_setup,
};
use hyperswitch_domain_models::{mandates, payment_address};
use payment_methods::helpers::StorageErrorExt;
use rustc_hash::FxHashSet;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use storage_impl::redis::cache;

#[cfg(feature = "payouts")]
use super::payouts;
use super::{
    errors::RouterResult,
    payments::{
        routing::{
            utils::*,
            {self as payments_routing},
        },
        OperationSessionGetters,
    },
};
#[cfg(feature = "v1")]
use crate::utils::ValueExt;
#[cfg(feature = "v2")]
use crate::{core::admin, utils::ValueExt};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResponse},
        metrics,
        payments::routing::get_active_mca_ids,
        utils as core_utils,
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
        transaction_type: enums::TransactionType,
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
            algorithm_for: transaction_type,
            decision_engine_routing_id: None,
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
    platform: domain::Platform,
    profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
    query_params: RoutingRetrieveQuery,
    transaction_type: enums::TransactionType,
) -> RouterResponse<routing_types::RoutingKind> {
    metrics::ROUTING_MERCHANT_DICTIONARY_RETRIEVE.add(1, &[]);

    let routing_metadata: Vec<diesel_models::routing_algorithm::RoutingProfileMetadata> = state
        .store
        .list_routing_algorithm_metadata_by_merchant_id_transaction_type(
            platform.get_processor().get_account().get_id(),
            &transaction_type,
            i64::from(query_params.limit.unwrap_or_default()),
            i64::from(query_params.offset.unwrap_or_default()),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
    let routing_metadata = super::utils::filter_objects_based_on_profile_id_list(
        profile_id_list.clone(),
        routing_metadata,
    );

    let mut result = routing_metadata
        .into_iter()
        .map(ForeignInto::foreign_into)
        .collect::<Vec<_>>();

    if let Some(profile_ids) = profile_id_list {
        let mut de_result: Vec<routing_types::RoutingDictionaryRecord> = vec![];
        // DE_TODO: need to replace this with batch API call to reduce the number of network calls
        for profile_id in &profile_ids {
            let list_request = ListRountingAlgorithmsRequest {
                created_by: profile_id.get_string_repr().to_string(),
            };
            list_de_euclid_routing_algorithms(&state, list_request)
                .await
                .map_err(|e| {
                    router_env::logger::error!(decision_engine_error=?e, "decision_engine_euclid");
                })
                .ok() // Avoid throwing error if Decision Engine is not available or other errors
                .map(|mut de_routing| de_result.append(&mut de_routing));
            // filter de_result based on transaction type
            de_result.retain(|record| record.algorithm_for == Some(transaction_type));
            // append dynamic routing algorithms to de_result
            de_result.append(
                &mut result
                    .clone()
                    .into_iter()
                    .filter(|record: &routing_types::RoutingDictionaryRecord| {
                        record.kind == routing_types::RoutingAlgorithmKind::Dynamic
                    })
                    .collect::<Vec<_>>(),
            );
        }
        compare_and_log_result(
            de_result.clone(),
            result.clone(),
            "list_routing".to_string(),
        );
        result =
            build_list_routing_result(&state, platform, &result, &de_result, profile_ids.clone())
                .await?;
    }

    metrics::ROUTING_MERCHANT_DICTIONARY_RETRIEVE_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::RoutingKind::RoutingAlgorithm(result),
    ))
}

async fn build_list_routing_result(
    state: &SessionState,
    platform: domain::Platform,
    hs_results: &[routing_types::RoutingDictionaryRecord],
    de_results: &[routing_types::RoutingDictionaryRecord],
    profile_ids: Vec<common_utils::id_type::ProfileId>,
) -> RouterResult<Vec<routing_types::RoutingDictionaryRecord>> {
    let db = state.store.as_ref();
    let mut list_result: Vec<routing_types::RoutingDictionaryRecord> = vec![];
    for profile_id in profile_ids.iter() {
        let by_profile =
            |rec: &&routing_types::RoutingDictionaryRecord| &rec.profile_id == profile_id;
        let de_result_for_profile = de_results.iter().filter(by_profile).cloned().collect();
        let hs_result_for_profile = hs_results.iter().filter(by_profile).cloned().collect();
        let business_profile = core_utils::validate_and_get_business_profile(
            db,
            platform.get_processor().get_key_store(),
            Some(profile_id),
            platform.get_processor().get_account().get_id(),
        )
        .await?
        .get_required_value("Profile")
        .change_context(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?;

        list_result.append(
            &mut select_routing_result(
                state,
                &business_profile,
                hs_result_for_profile,
                de_result_for_profile,
            )
            .await,
        );
    }
    Ok(list_result)
}

#[cfg(feature = "v2")]
pub async fn create_routing_algorithm_under_profile(
    state: SessionState,
    platform: domain::Platform,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    request: routing_types::RoutingConfigRequest,
    transaction_type: enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_CREATE_REQUEST_RECEIVED.add(1, &[]);
    let db = &*state.store;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&request.profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")?;
    let merchant_id = platform.get_processor().get_account().get_id();
    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;
    let all_mcas = state
        .store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            merchant_id,
            true,
            platform.get_processor().get_key_store(),
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_id.get_string_repr().to_owned(),
        })?;

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
        platform.get_processor().get_account().get_id(),
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

    metrics::ROUTING_CREATE_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(new_record))
}

#[cfg(feature = "v1")]
pub async fn create_routing_algorithm_under_profile(
    state: SessionState,
    platform: domain::Platform,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    request: routing_types::RoutingConfigRequest,
    transaction_type: enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    use api_models::routing::StaticRoutingAlgorithm as EuclidAlgorithm;

    metrics::ROUTING_CREATE_REQUEST_RECEIVED.add(1, &[]);
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
        .clone()
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
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")?;

    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;

    if algorithm.should_validate_connectors_in_routing_config() {
        helpers::validate_connectors_in_routing_config(
            &state,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().get_id(),
            &profile_id,
            &algorithm,
        )
        .await?;
    }

    let mut decision_engine_routing_id: Option<String> = None;

    if let Some(euclid_algorithm) = request.algorithm.clone() {
        let maybe_static_algorithm: Option<StaticRoutingAlgorithm> = match euclid_algorithm {
            EuclidAlgorithm::Advanced(program) => match program.try_into() {
                Ok(internal_program) => Some(StaticRoutingAlgorithm::Advanced(internal_program)),
                Err(e) => {
                    router_env::logger::error!(decision_engine_error = ?e, "decision_engine_euclid");
                    None
                }
            },
            EuclidAlgorithm::Single(conn) => {
                Some(StaticRoutingAlgorithm::Single(Box::new(conn.into())))
            }
            EuclidAlgorithm::Priority(connectors) => {
                let converted: Vec<ConnectorInfo> =
                    connectors.into_iter().map(Into::into).collect();
                Some(StaticRoutingAlgorithm::Priority(converted))
            }
            EuclidAlgorithm::VolumeSplit(splits) => {
                let converted: Vec<VolumeSplit<ConnectorInfo>> =
                    splits.into_iter().map(Into::into).collect();
                Some(StaticRoutingAlgorithm::VolumeSplit(converted))
            }
            EuclidAlgorithm::ThreeDsDecisionRule(_) => {
                router_env::logger::error!(
                    "decision_engine_euclid: ThreeDsDecisionRules are not yet implemented"
                );
                None
            }
        };

        if let Some(static_algorithm) = maybe_static_algorithm {
            let routing_rule = RoutingRule {
                rule_id: Some(algorithm_id.clone().get_string_repr().to_owned()),
                name: name.to_string(),
                description: Some(description.clone()),
                created_by: profile_id.get_string_repr().to_string(),
                algorithm: static_algorithm,
                algorithm_for: transaction_type.into(),
                metadata: Some(RoutingMetadata {
                    kind: algorithm.get_kind().foreign_into(),
                }),
            };

            match create_de_euclid_routing_algo(&state, &routing_rule).await {
                Ok(id) => {
                    decision_engine_routing_id = Some(id);
                }
                Err(e)
                    if matches!(
                        e.current_context(),
                        errors::RoutingError::DecisionEngineValidationError(_)
                    ) =>
                {
                    if let errors::RoutingError::DecisionEngineValidationError(msg) =
                        e.current_context()
                    {
                        router_env::logger::error!(
                            decision_engine_euclid_error = ?msg,
                            decision_engine_euclid_request = ?routing_rule,
                            "failed to create rule in decision_engine with validation error"
                        );
                    }
                }
                Err(e) => {
                    router_env::logger::error!(
                        decision_engine_euclid_error = ?e,
                        decision_engine_euclid_request = ?routing_rule,
                        "failed to create rule in decision_engine"
                    );
                }
            }
        }
    }

    if decision_engine_routing_id.is_some() {
        router_env::logger::info!(routing_flow=?"create_euclid_routing_algorithm", is_equal=?"true", "decision_engine_euclid");
    } else {
        router_env::logger::info!(routing_flow=?"create_euclid_routing_algorithm", is_equal=?"false", "decision_engine_euclid");
    }

    let timestamp = common_utils::date_time::now();
    let algo = RoutingAlgorithm {
        algorithm_id: algorithm_id.clone(),
        profile_id,
        merchant_id: platform.get_processor().get_account().get_id().to_owned(),
        name: name.to_string(),
        description: Some(description.clone()),
        kind: algorithm.get_kind().foreign_into(),
        algorithm_data: serde_json::json!(algorithm),
        created_at: timestamp,
        modified_at: timestamp,
        algorithm_for: transaction_type.to_owned(),
        decision_engine_routing_id,
    };
    let record = db
        .insert_routing_algorithm(algo)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let new_record = record.foreign_into();

    metrics::ROUTING_CREATE_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(new_record))
}

#[cfg(feature = "v2")]
pub async fn link_routing_config_under_profile(
    state: SessionState,
    platform: domain::Platform,
    profile_id: common_utils::id_type::ProfileId,
    algorithm_id: common_utils::id_type::RoutingId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_LINK_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let routing_algorithm = RoutingAlgorithmUpdate::fetch_routing_algo(
        platform.get_processor().get_account().get_id(),
        &algorithm_id,
        db,
    )
    .await?;

    utils::when(routing_algorithm.0.profile_id != profile_id, || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "Profile Id is invalid for the routing config".to_string(),
        })
    })?;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
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
            platform.get_processor().get_key_store(),
            algorithm_id,
            transaction_type,
        )
        .await?;

    metrics::ROUTING_LINK_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_algorithm.0.foreign_into(),
    ))
}

#[cfg(feature = "v1")]
pub async fn link_routing_config(
    state: SessionState,
    platform: domain::Platform,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    algorithm_id: common_utils::id_type::RoutingId,
    transaction_type: enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_LINK_CONFIG.add(1, &[]);
    let db = state.store.as_ref();

    let routing_algorithm = db
        .find_routing_algorithm_by_algorithm_id_merchant_id(
            &algorithm_id,
            platform.get_processor().get_account().get_id(),
        )
        .await
        .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&routing_algorithm.profile_id),
        platform.get_processor().get_account().get_id(),
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
                ) || matches!(
                    dynamic_routing_ref.elimination_routing_algorithm,
                    Some(routing::EliminationRoutingAlgorithm {
                        algorithm_id_with_timestamp:
                        routing_types::DynamicAlgorithmWithTimestamp {
                            algorithm_id: Some(ref id),
                            timestamp: _
                        },
                        enabled_feature: _
                    }) if id == &algorithm_id
                ) || matches!(
                    dynamic_routing_ref.contract_based_routing,
                    Some(routing::ContractRoutingAlgorithm {
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

            if routing_algorithm.name == helpers::SUCCESS_BASED_DYNAMIC_ROUTING_ALGORITHM {
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
                routing_types::DynamicRoutingType::SuccessRateBasedRouting,
            );

                // Call to DE here to update SR configs
                #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
                {
                    if state.conf.open_router.dynamic_routing_enabled {
                        let existing_config = helpers::get_decision_engine_active_dynamic_routing_algorithm(
                        &state,
                        business_profile.get_id(),
                        api_models::open_router::DecisionEngineDynamicAlgorithmType::SuccessRate,
                    )
                    .await;

                        if let Ok(Some(_config)) = existing_config {
                            update_decision_engine_dynamic_routing_setup(
                            &state,
                            business_profile.get_id(),
                            routing_algorithm.algorithm_data.clone(),
                            routing_types::DynamicRoutingType::SuccessRateBasedRouting,
                            &mut dynamic_routing_ref,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Failed to update the success rate routing config in Decision Engine",
                        )?;
                        } else {
                            let data: routing_types::SuccessBasedRoutingConfig =
                            routing_algorithm.algorithm_data
                                .clone()
                                .parse_value("SuccessBasedRoutingConfig")
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable(
                                    "unable to deserialize SuccessBasedRoutingConfig from routing algorithm data",
                                )?;

                            enable_decision_engine_dynamic_routing_setup(
                            &state,
                            business_profile.get_id(),
                            routing_types::DynamicRoutingType::SuccessRateBasedRouting,
                            &mut dynamic_routing_ref,
                            Some(routing_types::DynamicRoutingPayload::SuccessBasedRoutingPayload(data)),
                        )
                        .await
                        .map_err(|err| match err.current_context() {
                            errors::ApiErrorResponse::GenericNotFoundError {..}=> {
                                err.change_context(errors::ApiErrorResponse::ConfigNotFound)
                                .attach_printable("Decision engine config not found")
                            }
                            _ => err
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("Unable to setup decision engine dynamic routing"),
                        })?;
                        }
                    }
                }
            } else if routing_algorithm.name == helpers::ELIMINATION_BASED_DYNAMIC_ROUTING_ALGORITHM
            {
                dynamic_routing_ref.update_algorithm_id(
                algorithm_id,
                dynamic_routing_ref
                    .elimination_routing_algorithm
                    .clone()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "missing elimination_routing_algorithm in dynamic_algorithm_ref from business_profile table",
                    )?
                    .enabled_feature,
                routing_types::DynamicRoutingType::EliminationRouting,
            );
                #[cfg(all(feature = "dynamic_routing", feature = "v1"))]
                {
                    if state.conf.open_router.dynamic_routing_enabled {
                        let existing_config = helpers::get_decision_engine_active_dynamic_routing_algorithm(
                            &state,
                            business_profile.get_id(),
                            api_models::open_router::DecisionEngineDynamicAlgorithmType::Elimination,
                        )
                        .await;

                        if let Ok(Some(_config)) = existing_config {
                            update_decision_engine_dynamic_routing_setup(
                                &state,
                                business_profile.get_id(),
                                routing_algorithm.algorithm_data.clone(),
                                routing_types::DynamicRoutingType::EliminationRouting,
                                &mut dynamic_routing_ref,
                            )
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable(
                                "Failed to update the elimination routing config in Decision Engine",
                            )?;
                        } else {
                            let data: routing_types::EliminationRoutingConfig =
                                routing_algorithm.algorithm_data
                                    .clone()
                                    .parse_value("EliminationRoutingConfig")
                                    .change_context(errors::ApiErrorResponse::InternalServerError)
                                    .attach_printable(
                                        "unable to deserialize EliminationRoutingConfig from routing algorithm data",
                                    )?;

                            enable_decision_engine_dynamic_routing_setup(
                                &state,
                                business_profile.get_id(),
                                routing_types::DynamicRoutingType::EliminationRouting,
                                &mut dynamic_routing_ref,
                                Some(
                                    routing_types::DynamicRoutingPayload::EliminationRoutingPayload(
                                        data,
                                    ),
                                ),
                            )
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Unable to setup decision engine dynamic routing")?;
                        }
                    }
                }
            } else if routing_algorithm.name == helpers::CONTRACT_BASED_DYNAMIC_ROUTING_ALGORITHM {
                dynamic_routing_ref.update_algorithm_id(
                algorithm_id,
                dynamic_routing_ref
                    .contract_based_routing
                    .clone()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "missing contract_based_routing in dynamic_algorithm_ref from business_profile table",
                    )?
                    .enabled_feature,
                routing_types::DynamicRoutingType::ContractBasedRouting,
            );
            }

            helpers::update_business_profile_active_dynamic_algorithm_ref(
                db,
                platform.get_processor().get_key_store(),
                business_profile.clone(),
                dynamic_routing_ref,
            )
            .await?;
        }
        diesel_models::enums::RoutingAlgorithmKind::Single
        | diesel_models::enums::RoutingAlgorithmKind::Priority
        | diesel_models::enums::RoutingAlgorithmKind::Advanced
        | diesel_models::enums::RoutingAlgorithmKind::VolumeSplit
        | diesel_models::enums::RoutingAlgorithmKind::ThreeDsDecisionRule => {
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

            utils::when(routing_algorithm.algorithm_for != transaction_type, || {
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
                platform.get_processor().get_key_store(),
                business_profile.clone(),
                routing_ref,
                &transaction_type,
            )
            .await?;
        }
    };
    if let Some(euclid_routing_id) = routing_algorithm.decision_engine_routing_id.clone() {
        let routing_algo = ActivateRoutingConfigRequest {
            created_by: business_profile.get_id().get_string_repr().to_string(),
            routing_algorithm_id: euclid_routing_id,
        };
        let link_result = link_de_euclid_routing_algorithm(&state, routing_algo).await;
        match link_result {
            Ok(_) => {
                router_env::logger::info!(
                    routing_flow=?"link_routing_algorithm",
                    is_equal=?true,
                    "decision_engine_euclid"
                );
            }
            Err(e) => {
                router_env::logger::info!(
                    routing_flow=?"link_routing_algorithm",
                    is_equal=?false,
                    error=?e,
                    "decision_engine_euclid"
                );
            }
        }
    }

    // redact cgraph cache on rule activation
    helpers::redact_cgraph_cache(
        &state,
        platform.get_processor().get_account().get_id(),
        business_profile.get_id(),
    )
    .await?;

    // redact routing cache on rule activation
    helpers::redact_routing_cache(
        &state,
        platform.get_processor().get_account().get_id(),
        business_profile.get_id(),
    )
    .await?;

    metrics::ROUTING_LINK_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_algorithm.foreign_into(),
    ))
}

#[cfg(feature = "v2")]
pub async fn retrieve_routing_algorithm_from_algorithm_id(
    state: SessionState,
    platform: domain::Platform,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    algorithm_id: common_utils::id_type::RoutingId,
) -> RouterResponse<routing_types::MerchantRoutingAlgorithm> {
    metrics::ROUTING_RETRIEVE_CONFIG.add(1, &[]);
    let db = state.store.as_ref();

    let routing_algorithm = RoutingAlgorithmUpdate::fetch_routing_algo(
        platform.get_processor().get_account().get_id(),
        &algorithm_id,
        db,
    )
    .await?;
    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&routing_algorithm.0.profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;

    let response = routing_types::MerchantRoutingAlgorithm::foreign_try_from(routing_algorithm.0)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse routing algorithm")?;

    metrics::ROUTING_RETRIEVE_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(response))
}

#[cfg(feature = "v1")]
pub async fn retrieve_routing_algorithm_from_algorithm_id(
    state: SessionState,
    platform: domain::Platform,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    algorithm_id: common_utils::id_type::RoutingId,
) -> RouterResponse<routing_types::MerchantRoutingAlgorithm> {
    metrics::ROUTING_RETRIEVE_CONFIG.add(1, &[]);
    let db = state.store.as_ref();

    let routing_algorithm = db
        .find_routing_algorithm_by_algorithm_id_merchant_id(
            &algorithm_id,
            platform.get_processor().get_account().get_id(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&routing_algorithm.profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;

    let response = routing_types::MerchantRoutingAlgorithm::foreign_try_from(routing_algorithm)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse routing algorithm")?;

    metrics::ROUTING_RETRIEVE_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn unlink_routing_config_under_profile(
    state: SessionState,
    platform: domain::Platform,
    profile_id: common_utils::id_type::ProfileId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UNLINK_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")?;

    let routing_algo_id = match transaction_type {
        enums::TransactionType::Payment => business_profile.routing_algorithm_id.clone(),
        #[cfg(feature = "payouts")]
        enums::TransactionType::Payout => business_profile.payout_routing_algorithm_id.clone(),
        // TODO: Handle ThreeDsAuthentication Transaction Type for Three DS Decision Rule Algorithm configuration
        enums::TransactionType::ThreeDsAuthentication => todo!(),
    };

    if let Some(algorithm_id) = routing_algo_id {
        let record = RoutingAlgorithmUpdate::fetch_routing_algo(
            platform.get_processor().get_account().get_id(),
            &algorithm_id,
            db,
        )
        .await?;
        let response = record.0.foreign_into();
        admin::ProfileWrapper::new(business_profile)
            .update_profile_and_invalidate_routing_config_for_active_algorithm_id_update(
                db,
                key_manager_state,
                platform.get_processor().get_key_store(),
                algorithm_id,
                transaction_type,
            )
            .await?;
        metrics::ROUTING_UNLINK_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
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
    platform: domain::Platform,
    request: routing_types::RoutingConfigRequest,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    transaction_type: enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UNLINK_CONFIG.add(1, &[]);

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
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
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
                enums::TransactionType::ThreeDsAuthentication => {
                    business_profile.three_ds_decision_rule_algorithm.clone()
                }
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
                        platform.get_processor().get_key_store(),
                        business_profile.clone(),
                        routing_algorithm,
                        &transaction_type,
                    )
                    .await?;

                    // redact cgraph cache on rule activation
                    helpers::redact_cgraph_cache(
                        &state,
                        platform.get_processor().get_account().get_id(),
                        business_profile.get_id(),
                    )
                    .await?;

                    // redact routing cache on rule activation
                    helpers::redact_routing_cache(
                        &state,
                        platform.get_processor().get_account().get_id(),
                        business_profile.get_id(),
                    )
                    .await?;

                    metrics::ROUTING_UNLINK_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
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
    platform: domain::Platform,
    profile_id: common_utils::id_type::ProfileId,
    updated_list_of_connectors: Vec<routing_types::RoutableConnectorChoice>,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_UPDATE_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
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
            platform.get_processor().get_key_store(),
        )
        .await?;

    metrics::ROUTING_UPDATE_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        updated_list_of_connectors,
    ))
}

#[cfg(feature = "v1")]
pub async fn update_default_routing_config(
    state: SessionState,
    platform: domain::Platform,
    updated_config: Vec<routing_types::RoutableConnectorChoice>,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_UPDATE_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let default_config = helpers::get_merchant_default_config(
        db,
        platform
            .get_processor()
            .get_account()
            .get_id()
            .get_string_repr(),
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
        platform
            .get_processor()
            .get_account()
            .get_id()
            .get_string_repr(),
        updated_config.clone(),
        transaction_type,
    )
    .await?;

    metrics::ROUTING_UPDATE_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(updated_config))
}

#[cfg(feature = "v2")]
pub async fn retrieve_default_fallback_algorithm_for_profile(
    state: SessionState,
    platform: domain::Platform,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")?;

    let connectors_choice = admin::ProfileWrapper::new(profile)
        .get_default_fallback_list_of_connector_under_profile()?;

    metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(connectors_choice))
}

#[cfg(feature = "v1")]
pub async fn retrieve_default_routing_config(
    state: SessionState,
    profile_id: Option<common_utils::id_type::ProfileId>,
    platform: domain::Platform,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let id = profile_id
        .map(|profile_id| profile_id.get_string_repr().to_owned())
        .unwrap_or_else(|| {
            platform
                .get_processor()
                .get_account()
                .get_id()
                .get_string_repr()
                .to_string()
        });

    helpers::get_merchant_default_config(db, &id, transaction_type)
        .await
        .map(|conn_choice| {
            metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
            service_api::ApplicationResponse::Json(conn_choice)
        })
}

#[cfg(feature = "v2")]
pub async fn retrieve_routing_config_under_profile(
    state: SessionState,
    platform: domain::Platform,
    query_params: RoutingRetrieveQuery,
    profile_id: common_utils::id_type::ProfileId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::LinkedRoutingConfigRetrieveResponse> {
    metrics::ROUTING_RETRIEVE_LINK_CONFIG.add(1, &[]);
    let db = state.store.as_ref();

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
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

    metrics::ROUTING_RETRIEVE_LINK_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::LinkedRoutingConfigRetrieveResponse::ProfileBased(active_algorithms),
    ))
}

#[cfg(feature = "v1")]
pub async fn retrieve_linked_routing_config(
    state: SessionState,
    platform: domain::Platform,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    query_params: routing_types::RoutingRetrieveLinkQuery,
    transaction_type: enums::TransactionType,
) -> RouterResponse<routing_types::LinkedRoutingConfigRetrieveResponse> {
    metrics::ROUTING_RETRIEVE_LINK_CONFIG.add(1, &[]);

    let db = state.store.as_ref();
    let merchant_key_store = platform.get_processor().get_key_store();
    let merchant_id = platform.get_processor().get_account().get_id();

    // Get business profiles
    let business_profiles = if let Some(profile_id) = query_params.profile_id {
        core_utils::validate_and_get_business_profile(
            db,
            merchant_key_store,
            Some(&profile_id),
            merchant_id,
        )
        .await?
        .map(|profile| vec![profile])
        .get_required_value("Profile")
        .change_context(errors::ApiErrorResponse::ProfileNotFound {
            id: profile_id.get_string_repr().to_owned(),
        })?
    } else {
        let business_profile = db
            .list_profile_by_merchant_id(merchant_key_store, merchant_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
        core_utils::filter_objects_based_on_profile_id_list(
            authentication_profile_id.map(|profile_id| vec![profile_id]),
            business_profile,
        )
    };

    let mut active_algorithms = Vec::new();

    for business_profile in business_profiles {
        let profile_id = business_profile.get_id();

        // Handle static routing algorithm
        let routing_ref: routing_types::RoutingAlgorithmRef = match transaction_type {
            enums::TransactionType::Payment => &business_profile.routing_algorithm,
            #[cfg(feature = "payouts")]
            enums::TransactionType::Payout => &business_profile.payout_routing_algorithm,
            enums::TransactionType::ThreeDsAuthentication => {
                &business_profile.three_ds_decision_rule_algorithm
            }
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
                    profile_id,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
            let hs_records: Vec<routing_types::RoutingDictionaryRecord> =
                vec![record.foreign_into()];
            let de_records = retrieve_decision_engine_active_rules(
                &state,
                &transaction_type,
                profile_id.clone(),
                hs_records.clone(),
            )
            .await;
            compare_and_log_result(
                de_records.clone(),
                hs_records.clone(),
                "list_active_routing".to_string(),
            );
            active_algorithms.append(
                &mut select_routing_result(&state, &business_profile, hs_records, de_records).await,
            );
        }

        // Handle dynamic routing algorithms
        let dynamic_routing_ref: routing_types::DynamicRoutingAlgorithmRef = business_profile
            .dynamic_routing_algorithm
            .clone()
            .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to deserialize dynamic routing algorithm ref from business profile",
            )?
            .unwrap_or_default();

        // Collect all dynamic algorithm IDs
        let mut dynamic_algorithm_ids = Vec::new();

        if let Some(sba) = &dynamic_routing_ref.success_based_algorithm {
            if let Some(id) = &sba.algorithm_id_with_timestamp.algorithm_id {
                dynamic_algorithm_ids.push(id.clone());
            }
        }
        if let Some(era) = &dynamic_routing_ref.elimination_routing_algorithm {
            if let Some(id) = &era.algorithm_id_with_timestamp.algorithm_id {
                dynamic_algorithm_ids.push(id.clone());
            }
        }
        if let Some(cbr) = &dynamic_routing_ref.contract_based_routing {
            if let Some(id) = &cbr.algorithm_id_with_timestamp.algorithm_id {
                dynamic_algorithm_ids.push(id.clone());
            }
        }

        // Fetch all dynamic algorithms
        for algorithm_id in dynamic_algorithm_ids {
            let record = db
                .find_routing_algorithm_metadata_by_algorithm_id_profile_id(
                    &algorithm_id,
                    profile_id,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
            if record.algorithm_for == transaction_type {
                active_algorithms.push(record.foreign_into());
            }
        }
    }

    metrics::ROUTING_RETRIEVE_LINK_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::LinkedRoutingConfigRetrieveResponse::ProfileBased(active_algorithms),
    ))
}

pub async fn retrieve_decision_engine_active_rules(
    state: &SessionState,
    transaction_type: &enums::TransactionType,
    profile_id: common_utils::id_type::ProfileId,
    hs_records: Vec<routing_types::RoutingDictionaryRecord>,
) -> Vec<routing_types::RoutingDictionaryRecord> {
    let mut de_records =
        list_de_euclid_active_routing_algorithm(state, profile_id.get_string_repr().to_owned())
            .await
            .map_err(|e| {
                router_env::logger::error!(?e, "Failed to list DE Euclid active routing algorithm");
            })
            .ok() // Avoid throwing error if Decision Engine is not available or other errors thrown
            .unwrap_or_default();
    // Use Hs records to list the dynamic algorithms as DE is not supporting dynamic algorithms in HS standard
    let mut dynamic_algos = hs_records
        .into_iter()
        .filter(|record| record.kind == routing_types::RoutingAlgorithmKind::Dynamic)
        .collect::<Vec<_>>();
    de_records.append(&mut dynamic_algos);
    de_records
        .into_iter()
        .filter(|r| r.algorithm_for == Some(*transaction_type))
        .collect::<Vec<_>>()
}
// List all the default fallback algorithms under all the profile under a merchant
pub async fn retrieve_default_routing_config_for_profiles(
    state: SessionState,
    platform: domain::Platform,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::ProfileDefaultRoutingConfig>> {
    metrics::ROUTING_RETRIEVE_CONFIG_FOR_PROFILE.add(1, &[]);
    let db = state.store.as_ref();

    let all_profiles = db
        .list_profile_by_merchant_id(
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().get_id(),
        )
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

    metrics::ROUTING_RETRIEVE_CONFIG_FOR_PROFILE_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(default_configs))
}

pub async fn update_default_routing_config_for_profile(
    state: SessionState,
    platform: domain::Platform,
    updated_config: Vec<routing_types::RoutableConnectorChoice>,
    profile_id: common_utils::id_type::ProfileId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::ProfileDefaultRoutingConfig> {
    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE.add(1, &[]);

    let db = state.store.as_ref();

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
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

    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::ProfileDefaultRoutingConfig {
            profile_id: business_profile.get_id().to_owned(),
            connectors: updated_config,
        },
    ))
}

// Toggle the specific routing type as well as add the default configs in RoutingAlgorithm table
// and update the same in business profile table.

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn toggle_specific_dynamic_routing(
    state: SessionState,
    platform: domain::Platform,
    feature_to_enable: routing::DynamicRoutingFeatures,
    profile_id: common_utils::id_type::ProfileId,
    dynamic_routing_type: routing::DynamicRoutingType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_CREATE_REQUEST_RECEIVED.add(
        1,
        router_env::metric_attributes!(
            ("profile_id", profile_id.clone()),
            ("algorithm_type", dynamic_routing_type.to_string())
        ),
    );
    let db = state.store.as_ref();

    let business_profile: domain::Profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: profile_id.get_string_repr().to_owned(),
    })?;

    let dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef = business_profile
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
        routing::DynamicRoutingFeatures::Metrics
        | routing::DynamicRoutingFeatures::DynamicConnectorSelection => {
            // occurs when algorithm is already present in the db
            // 1. If present with same feature then return response as already enabled
            // 2. Else update the feature and persist the same on db
            // 3. If not present in db then create a new default entry
            Box::pin(helpers::enable_dynamic_routing_algorithm(
                &state,
                platform.get_processor().get_key_store().clone(),
                business_profile,
                feature_to_enable,
                dynamic_routing_algo_ref,
                dynamic_routing_type,
                None,
            ))
            .await
        }
        routing::DynamicRoutingFeatures::None => {
            // disable specific dynamic routing for the requested profile
            helpers::disable_dynamic_routing_algorithm(
                &state,
                platform.get_processor().get_key_store().clone(),
                business_profile,
                dynamic_routing_algo_ref,
                dynamic_routing_type,
            )
            .await
        }
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn create_specific_dynamic_routing(
    state: SessionState,
    platform: domain::Platform,
    feature_to_enable: routing::DynamicRoutingFeatures,
    profile_id: common_utils::id_type::ProfileId,
    dynamic_routing_type: routing::DynamicRoutingType,
    payload: Option<routing_types::DynamicRoutingPayload>,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_CREATE_REQUEST_RECEIVED.add(
        1,
        router_env::metric_attributes!(
            ("profile_id", profile_id.clone()),
            ("algorithm_type", dynamic_routing_type.to_string())
        ),
    );
    let db = state.store.as_ref();

    let business_profile: domain::Profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: profile_id.get_string_repr().to_owned(),
    })?;

    let dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef = business_profile
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
        routing::DynamicRoutingFeatures::Metrics
        | routing::DynamicRoutingFeatures::DynamicConnectorSelection => {
            Box::pin(helpers::enable_dynamic_routing_algorithm(
                &state,
                platform.get_processor().get_key_store().clone(),
                business_profile,
                feature_to_enable,
                dynamic_routing_algo_ref,
                dynamic_routing_type,
                payload,
            ))
            .await
        }
        routing::DynamicRoutingFeatures::None => {
            // disable specific dynamic routing for the requested profile
            helpers::disable_dynamic_routing_algorithm(
                &state,
                platform.get_processor().get_key_store().clone(),
                business_profile,
                dynamic_routing_algo_ref,
                dynamic_routing_type,
            )
            .await
        }
    }
}

#[cfg(feature = "v1")]
pub async fn configure_dynamic_routing_volume_split(
    state: SessionState,
    platform: domain::Platform,
    profile_id: common_utils::id_type::ProfileId,
    routing_info: routing::RoutingVolumeSplit,
) -> RouterResponse<routing::RoutingVolumeSplit> {
    metrics::ROUTING_CREATE_REQUEST_RECEIVED.add(
        1,
        router_env::metric_attributes!(("profile_id", profile_id.clone())),
    );
    let db = state.store.as_ref();

    utils::when(
        routing_info.split > crate::consts::DYNAMIC_ROUTING_MAX_VOLUME,
        || {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Dynamic routing volume split should be less than 100".to_string(),
            })
        },
    )?;

    let business_profile: domain::Profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: profile_id.get_string_repr().to_owned(),
    })?;

    let mut dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef = business_profile
        .dynamic_routing_algorithm
        .clone()
        .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "unable to deserialize dynamic routing algorithm ref from business profile",
        )?
        .unwrap_or_default();

    dynamic_routing_algo_ref.update_volume_split(Some(routing_info.split));

    helpers::update_business_profile_active_dynamic_algorithm_ref(
        db,
        platform.get_processor().get_key_store(),
        business_profile.clone(),
        dynamic_routing_algo_ref.clone(),
    )
    .await?;

    Ok(service_api::ApplicationResponse::Json(routing_info))
}

#[cfg(feature = "v1")]
pub async fn retrieve_dynamic_routing_volume_split(
    state: SessionState,
    platform: domain::Platform,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<routing_types::RoutingVolumeSplitResponse> {
    let db = state.store.as_ref();

    let business_profile: domain::Profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: profile_id.get_string_repr().to_owned(),
    })?;

    let dynamic_routing_algo_ref: routing_types::DynamicRoutingAlgorithmRef = business_profile
        .dynamic_routing_algorithm
        .clone()
        .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "unable to deserialize dynamic routing algorithm ref from business profile",
        )?
        .unwrap_or_default();

    let resp = routing_types::RoutingVolumeSplitResponse {
        split: dynamic_routing_algo_ref
            .dynamic_routing_volume_split
            .unwrap_or_default(),
    };

    Ok(service_api::ApplicationResponse::Json(resp))
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn success_based_routing_update_configs(
    state: SessionState,
    request: routing_types::SuccessBasedRoutingConfig,
    algorithm_id: common_utils::id_type::RoutingId,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE.add(
        1,
        router_env::metric_attributes!(
            ("profile_id", profile_id.clone()),
            (
                "algorithm_type",
                routing::DynamicRoutingType::SuccessRateBasedRouting.to_string()
            )
        ),
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
        algorithm_data: serde_json::json!(config_to_update.clone()),
        created_at: timestamp,
        modified_at: timestamp,
        algorithm_for: dynamic_routing_algo_to_update.algorithm_for,
        decision_engine_routing_id: None,
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
    let _ = cache::redact_from_redis_and_publish(
        state.store.get_cache_store().as_ref(),
        cache_entries_to_redact,
    )
    .await
    .map_err(|e| router_env::logger::error!("unable to publish into the redact channel for evicting the success based routing config cache {e:?}"));

    let new_record = record.foreign_into();

    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE_SUCCESS_RESPONSE.add(
        1,
        router_env::metric_attributes!(("profile_id", profile_id.clone())),
    );

    if !state.conf.open_router.dynamic_routing_enabled {
        state
            .grpc_client
            .dynamic_routing
            .as_ref()
            .async_map(|dr_client| async {
                dr_client
                    .success_rate_client
                    .invalidate_success_rate_routing_keys(
                        profile_id.get_string_repr().into(),
                        state.get_grpc_headers(),
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to invalidate the routing keys")
            })
            .await
            .transpose()?;
    }

    Ok(service_api::ApplicationResponse::Json(new_record))
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn elimination_routing_update_configs(
    state: SessionState,
    request: routing_types::EliminationRoutingConfig,
    algorithm_id: common_utils::id_type::RoutingId,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE.add(
        1,
        router_env::metric_attributes!(
            ("profile_id", profile_id.clone()),
            (
                "algorithm_type",
                routing::DynamicRoutingType::EliminationRouting.to_string()
            )
        ),
    );

    let db = state.store.as_ref();

    let dynamic_routing_algo_to_update = db
        .find_routing_algorithm_by_profile_id_algorithm_id(&profile_id, &algorithm_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let mut config_to_update: routing::EliminationRoutingConfig = dynamic_routing_algo_to_update
        .algorithm_data
        .parse_value::<routing::EliminationRoutingConfig>("EliminationRoutingConfig")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "unable to deserialize algorithm data from routing table into EliminationRoutingConfig",
        )?;

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
        decision_engine_routing_id: None,
    };

    let record = db
        .insert_routing_algorithm(algo)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to insert record in routing algorithm table")?;

    // redact cache for elimination routing configs
    let cache_key = format!(
        "{}_{}",
        profile_id.get_string_repr(),
        algorithm_id.get_string_repr()
    );
    let cache_entries_to_redact = vec![cache::CacheKind::EliminationBasedDynamicRoutingCache(
        cache_key.into(),
    )];

    cache::redact_from_redis_and_publish(
        state.store.get_cache_store().as_ref(),
        cache_entries_to_redact,
    )
    .await
    .map_err(|e| router_env::logger::error!("unable to publish into the redact channel for evicting the elimination routing config cache {e:?}")).ok();

    let new_record = record.foreign_into();

    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE_SUCCESS_RESPONSE.add(
        1,
        router_env::metric_attributes!(("profile_id", profile_id.clone())),
    );

    if !state.conf.open_router.dynamic_routing_enabled {
        state
            .grpc_client
            .dynamic_routing
            .as_ref()
            .async_map(|dr_client| async {
                dr_client
                    .elimination_based_client
                    .invalidate_elimination_bucket(
                        profile_id.get_string_repr().into(),
                        state.get_grpc_headers(),
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to invalidate the elimination routing keys")
            })
            .await
            .transpose()?;
    }

    Ok(service_api::ApplicationResponse::Json(new_record))
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn contract_based_dynamic_routing_setup(
    state: SessionState,
    platform: domain::Platform,
    profile_id: common_utils::id_type::ProfileId,
    feature_to_enable: routing_types::DynamicRoutingFeatures,
    config: Option<routing_types::ContractBasedRoutingConfig>,
) -> RouterResult<service_api::ApplicationResponse<routing_types::RoutingDictionaryRecord>> {
    let db = state.store.as_ref();

    let business_profile: domain::Profile = core_utils::validate_and_get_business_profile(
        db,
        platform.get_processor().get_key_store(),
        Some(&profile_id),
        platform.get_processor().get_account().get_id(),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: profile_id.get_string_repr().to_owned(),
    })?;

    let mut dynamic_routing_algo_ref: Option<routing_types::DynamicRoutingAlgorithmRef> =
        business_profile
            .dynamic_routing_algorithm
            .clone()
            .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "unable to deserialize dynamic routing algorithm ref from business profile",
            )
            .ok()
            .flatten();

    utils::when(
        dynamic_routing_algo_ref
            .as_mut()
            .and_then(|algo| {
                algo.contract_based_routing.as_mut().map(|contract_algo| {
                    *contract_algo.get_enabled_features() == feature_to_enable
                        && contract_algo
                            .clone()
                            .get_algorithm_id_with_timestamp()
                            .algorithm_id
                            .is_some()
                })
            })
            .unwrap_or(false),
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Contract Routing with specified features is already enabled".to_string(),
            })
        },
    )?;

    if feature_to_enable == routing::DynamicRoutingFeatures::None {
        let algorithm = dynamic_routing_algo_ref
            .clone()
            .get_required_value("dynamic_routing_algo_ref")
            .attach_printable("Failed to get dynamic_routing_algo_ref")?;
        return helpers::disable_dynamic_routing_algorithm(
            &state,
            platform.get_processor().get_key_store().clone(),
            business_profile,
            algorithm,
            routing_types::DynamicRoutingType::ContractBasedRouting,
        )
        .await;
    }

    let config = config
        .get_required_value("ContractBasedRoutingConfig")
        .attach_printable("Failed to get ContractBasedRoutingConfig from request")?;

    let merchant_id = business_profile.merchant_id.clone();
    let algorithm_id = common_utils::generate_routing_id_of_default_length();
    let timestamp = common_utils::date_time::now();

    let algo = RoutingAlgorithm {
        algorithm_id: algorithm_id.clone(),
        profile_id: profile_id.clone(),
        merchant_id,
        name: helpers::CONTRACT_BASED_DYNAMIC_ROUTING_ALGORITHM.to_string(),
        description: None,
        kind: diesel_models::enums::RoutingAlgorithmKind::Dynamic,
        algorithm_data: serde_json::json!(config),
        created_at: timestamp,
        modified_at: timestamp,
        algorithm_for: common_enums::TransactionType::Payment,
        decision_engine_routing_id: None,
    };

    // 1. if dynamic_routing_algo_ref already present, insert contract based algo and disable success based
    // 2. if dynamic_routing_algo_ref is not present, create a new dynamic_routing_algo_ref with contract algo set up
    let final_algorithm = if let Some(mut algo) = dynamic_routing_algo_ref {
        algo.update_algorithm_id(
            algorithm_id,
            feature_to_enable,
            routing_types::DynamicRoutingType::ContractBasedRouting,
        );
        if feature_to_enable == routing::DynamicRoutingFeatures::DynamicConnectorSelection {
            algo.disable_algorithm_id(routing_types::DynamicRoutingType::SuccessRateBasedRouting);
        }
        algo
    } else {
        let contract_algo = routing_types::ContractRoutingAlgorithm {
            algorithm_id_with_timestamp: routing_types::DynamicAlgorithmWithTimestamp::new(Some(
                algorithm_id.clone(),
            )),
            enabled_feature: feature_to_enable,
        };
        routing_types::DynamicRoutingAlgorithmRef {
            success_based_algorithm: None,
            elimination_routing_algorithm: None,
            dynamic_routing_volume_split: None,
            contract_based_routing: Some(contract_algo),
            is_merchant_created_in_decision_engine: dynamic_routing_algo_ref
                .as_ref()
                .is_some_and(|algo| algo.is_merchant_created_in_decision_engine),
        }
    };

    // validate the contained mca_ids
    let mut contained_mca = Vec::new();
    if let Some(info_vec) = &config.label_info {
        for info in info_vec {
            utils::when(
                contained_mca.iter().any(|mca_id| mca_id == &info.mca_id),
                || {
                    Err(error_stack::Report::new(
                        errors::ApiErrorResponse::InvalidRequestData {
                            message: "Duplicate mca configuration received".to_string(),
                        },
                    ))
                },
            )?;

            contained_mca.push(info.mca_id.to_owned());
        }

        let validation_futures: Vec<_> = info_vec
            .iter()
            .map(|info| async {
                let mca_id = info.mca_id.clone();
                let label = info.label.clone();
                let mca = db
                    .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                        platform.get_processor().get_account().get_id(),
                        &mca_id,
                        platform.get_processor().get_key_store(),
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                        id: mca_id.get_string_repr().to_owned(),
                    })?;

                utils::when(mca.connector_name != label, || {
                    Err(error_stack::Report::new(
                        errors::ApiErrorResponse::InvalidRequestData {
                            message: "Incorrect mca configuration received".to_string(),
                        },
                    ))
                })?;

                Ok::<_, error_stack::Report<errors::ApiErrorResponse>>(())
            })
            .collect();

        futures::future::try_join_all(validation_futures).await?;
    }

    let record = db
        .insert_routing_algorithm(algo)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to insert record in routing algorithm table")?;

    helpers::update_business_profile_active_dynamic_algorithm_ref(
        db,
        platform.get_processor().get_key_store(),
        business_profile,
        final_algorithm,
    )
    .await?;

    let new_record = record.foreign_into();

    metrics::ROUTING_CREATE_SUCCESS_RESPONSE.add(
        1,
        router_env::metric_attributes!(("profile_id", profile_id.get_string_repr().to_string())),
    );
    Ok(service_api::ApplicationResponse::Json(new_record))
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn contract_based_routing_update_configs(
    state: SessionState,
    request: routing_types::ContractBasedRoutingConfig,
    platform: domain::Platform,
    algorithm_id: common_utils::id_type::RoutingId,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE.add(
        1,
        router_env::metric_attributes!(
            ("profile_id", profile_id.get_string_repr().to_owned()),
            (
                "algorithm_type",
                routing::DynamicRoutingType::ContractBasedRouting.to_string()
            )
        ),
    );
    let db = state.store.as_ref();

    let dynamic_routing_algo_to_update = db
        .find_routing_algorithm_by_profile_id_algorithm_id(&profile_id, &algorithm_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let mut config_to_update: routing::ContractBasedRoutingConfig = dynamic_routing_algo_to_update
        .algorithm_data
        .parse_value::<routing::ContractBasedRoutingConfig>("ContractBasedRoutingConfig")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to deserialize algorithm data from routing table into ContractBasedRoutingConfig")?;

    // validate the contained mca_ids
    let mut contained_mca = Vec::new();
    if let Some(info_vec) = &request.label_info {
        for info in info_vec {
            let mca = db
                .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                    platform.get_processor().get_account().get_id(),
                    &info.mca_id,
                    platform.get_processor().get_key_store(),
                )
                .await
                .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                    id: info.mca_id.get_string_repr().to_owned(),
                })?;

            utils::when(mca.connector_name != info.label, || {
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Incorrect mca configuration received".to_string(),
                })
            })?;

            utils::when(
                contained_mca.iter().any(|mca_id| mca_id == &info.mca_id),
                || {
                    Err(error_stack::Report::new(
                        errors::ApiErrorResponse::InvalidRequestData {
                            message: "Duplicate mca configuration received".to_string(),
                        },
                    ))
                },
            )?;

            contained_mca.push(info.mca_id.to_owned());
        }
    }

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
        decision_engine_routing_id: None,
    };
    let record = db
        .insert_routing_algorithm(algo)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to insert record in routing algorithm table")?;

    // redact cache for contract based routing configs
    let cache_key = format!(
        "{}_{}",
        profile_id.get_string_repr(),
        algorithm_id.get_string_repr()
    );
    let cache_entries_to_redact = vec![cache::CacheKind::ContractBasedDynamicRoutingCache(
        cache_key.into(),
    )];
    let _ = cache::redact_from_redis_and_publish(
        state.store.get_cache_store().as_ref(),
        cache_entries_to_redact,
    )
    .await
    .map_err(|e| router_env::logger::error!("unable to publish into the redact channel for evicting the contract based routing config cache {e:?}"));

    let new_record = record.foreign_into();

    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE_SUCCESS_RESPONSE.add(
        1,
        router_env::metric_attributes!(("profile_id", profile_id.get_string_repr().to_owned())),
    );

    state
        .grpc_client
        .dynamic_routing
        .as_ref()
        .async_map(|dr_client| async {
            dr_client
                .contract_based_client
                .invalidate_contracts(
                    profile_id.get_string_repr().into(),
                    state.get_grpc_headers(),
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to invalidate the contract based routing keys")
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

        let active_mca_ids = get_active_mca_ids(state, key_store)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let connectors = payments_routing::perform_cgraph_filtering(
            state,
            key_store,
            routable_connector_choice,
            backend_input,
            None,
            profile_id,
            &common_enums::TransactionType::Payment,
            &active_mca_ids,
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

pub async fn migrate_rules_for_profile(
    state: SessionState,
    platform: domain::Platform,
    query_params: routing_types::RuleMigrationQuery,
) -> RouterResult<routing_types::RuleMigrationResult> {
    use api_models::routing::StaticRoutingAlgorithm as EuclidAlgorithm;

    let profile_id = query_params.profile_id.clone();
    let db = state.store.as_ref();
    let merchant_key_store = platform.get_processor().get_key_store();
    let merchant_id = platform.get_processor().get_account().get_id();

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        merchant_key_store,
        Some(&profile_id),
        merchant_id,
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ProfileNotFound {
        id: profile_id.get_string_repr().to_owned(),
    })?;

    #[cfg(feature = "v1")]
    let active_payment_routing_ids: Vec<Option<common_utils::id_type::RoutingId>> = vec![
        business_profile
            .get_payment_routing_algorithm()
            .attach_printable("Failed to get payment routing algorithm")?
            .unwrap_or_default()
            .algorithm_id,
        business_profile
            .get_payout_routing_algorithm()
            .attach_printable("Failed to get payout routing algorithm")?
            .unwrap_or_default()
            .algorithm_id,
    ];

    #[cfg(feature = "v2")]
    let active_payment_routing_ids = [business_profile.routing_algorithm_id.clone()];

    let routing_metadatas = state
        .store
        .list_routing_algorithm_metadata_by_profile_id(
            &profile_id,
            i64::from(query_params.validated_limit()),
            i64::from(query_params.offset.unwrap_or_default()),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    let mut response_list = Vec::new();
    let mut error_list = Vec::new();

    let mut push_error = |algorithm_id, msg: String| {
        error_list.push(RuleMigrationError {
            profile_id: profile_id.clone(),
            algorithm_id,
            error: msg,
        });
    };

    for routing_metadata in routing_metadatas {
        let algorithm_id = routing_metadata.algorithm_id.clone();
        let algorithm = match db
            .find_routing_algorithm_by_profile_id_algorithm_id(&profile_id, &algorithm_id)
            .await
        {
            Ok(algo) => algo,
            Err(e) => {
                router_env::logger::error!(?e, ?algorithm_id, "Failed to fetch routing algorithm");
                push_error(algorithm_id, format!("Fetch error: {e:?}"));
                continue;
            }
        };

        let parsed_result = algorithm
            .algorithm_data
            .parse_value::<EuclidAlgorithm>("EuclidAlgorithm");

        let maybe_static_algorithm: Option<StaticRoutingAlgorithm> = match parsed_result {
            Ok(EuclidAlgorithm::Advanced(program)) => match program.try_into() {
                Ok(ip) => Some(StaticRoutingAlgorithm::Advanced(ip)),
                Err(e) => {
                    router_env::logger::error!(
                        ?e,
                        ?algorithm_id,
                        "Failed to convert advanced program"
                    );
                    push_error(algorithm_id.clone(), format!("Conversion error: {e:?}"));
                    None
                }
            },
            Ok(EuclidAlgorithm::Single(conn)) => {
                Some(StaticRoutingAlgorithm::Single(Box::new(conn.into())))
            }
            Ok(EuclidAlgorithm::Priority(connectors)) => Some(StaticRoutingAlgorithm::Priority(
                connectors.into_iter().map(Into::into).collect(),
            )),
            Ok(EuclidAlgorithm::VolumeSplit(splits)) => Some(StaticRoutingAlgorithm::VolumeSplit(
                splits.into_iter().map(Into::into).collect(),
            )),
            Ok(EuclidAlgorithm::ThreeDsDecisionRule(_)) => {
                router_env::logger::info!(
                    ?algorithm_id,
                    "Skipping 3DS rule migration (not supported yet)"
                );
                push_error(algorithm_id.clone(), "3DS migration not implemented".into());
                None
            }
            Err(e) => {
                router_env::logger::error!(?e, ?algorithm_id, "Failed to parse algorithm");
                push_error(algorithm_id.clone(), format!("Parse error: {e:?}"));
                None
            }
        };

        let Some(static_algorithm) = maybe_static_algorithm else {
            continue;
        };

        let routing_rule = RoutingRule {
            rule_id: Some(algorithm.algorithm_id.clone().get_string_repr().to_string()),
            name: algorithm.name.clone(),
            description: algorithm.description.clone(),
            created_by: profile_id.get_string_repr().to_string(),
            algorithm: static_algorithm,
            algorithm_for: algorithm.algorithm_for.into(),
            metadata: Some(RoutingMetadata {
                kind: algorithm.kind,
            }),
        };

        match create_de_euclid_routing_algo(&state, &routing_rule).await {
            Ok(decision_engine_routing_id) => {
                let mut is_active_rule = false;
                if active_payment_routing_ids.contains(&Some(algorithm.algorithm_id.clone())) {
                    link_de_euclid_routing_algorithm(
                        &state,
                        ActivateRoutingConfigRequest {
                            created_by: profile_id.get_string_repr().to_string(),
                            routing_algorithm_id: decision_engine_routing_id.clone(),
                        },
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("unable to link active routing algorithm")?;
                    is_active_rule = true;
                }
                response_list.push(RuleMigrationResponse {
                    profile_id: profile_id.clone(),
                    euclid_algorithm_id: algorithm.algorithm_id.clone(),
                    decision_engine_algorithm_id: decision_engine_routing_id,
                    is_active_rule,
                });
            }
            Err(err) => {
                router_env::logger::error!(
                    decision_engine_rule_migration_error = ?err,
                    algorithm_id = ?algorithm.algorithm_id,
                    "Failed to insert into decision engine"
                );
                push_error(
                    algorithm.algorithm_id.clone(),
                    format!("Insertion error: {err:?}"),
                );
            }
        }
    }

    Ok(routing_types::RuleMigrationResult {
        success: response_list,
        errors: error_list,
    })
}

pub async fn decide_gateway_open_router(
    state: SessionState,
    req_body: OpenRouterDecideGatewayRequest,
) -> RouterResponse<DecideGatewayResponse> {
    let response = if state.conf.open_router.dynamic_routing_enabled {
        SRApiClient::send_decision_engine_request(
            &state,
            Method::Post,
            "decide-gateway",
            Some(req_body),
            None,
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .response
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to perform decide gateway call with open router")?
    } else {
        Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Dynamic routing is not enabled")?
    };

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

pub async fn update_gateway_score_open_router(
    state: SessionState,
    req_body: UpdateScorePayload,
) -> RouterResponse<UpdateScoreResponse> {
    let response = if state.conf.open_router.dynamic_routing_enabled {
        SRApiClient::send_decision_engine_request(
            &state,
            Method::Post,
            "update-gateway-score",
            Some(req_body),
            None,
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .response
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to perform update gateway score call with open router")?
    } else {
        Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Dynamic routing is not enabled")?
    };

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}
