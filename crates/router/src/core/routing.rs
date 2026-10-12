pub mod helpers;
pub mod transformers;
use std::collections::{HashMap, HashSet};

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
use common_utils::{ext_traits::Encode, request::Method};
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
use hyperswitch_masking::Secret;
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
        OperationSessionGetters, OperationSessionSetters,
    },
};
#[cfg(feature = "v1")]
use crate::utils::ValueExt;
#[cfg(feature = "v2")]
use crate::{core::admin, db::StorageInterface, utils::ValueExt};
use crate::{
    core::{
        configs::dimension_state,
        errors::{self, CustomResult, RouterResponse},
        metrics, utils as core_utils,
    },
    routes::SessionState,
    services::api as service_api,
    types::{
        api, domain,
        storage::{self, enums as storage_enums},
        transformers::{ForeignFrom, ForeignInto, ForeignTryFrom},
    },
    utils::{self, OptionExt},
};

/// Describes the dashboard user to the Decision Engine: which part of the tree the session may
/// move within, and what it may do there. States the user's position rather than a list of scopes;
/// DE resolves it against its own synced tree. Falls back to a bare profile request if the role
/// cannot be read.
#[cfg(all(feature = "olap", feature = "v1"))]
async fn decision_engine_token_request(
    state: &SessionState,
    profile_id: &common_utils::id_type::ProfileId,
    user: crate::services::authentication::UserFromToken,
) -> api_models::open_router::MerchantTokenRequest {
    use api_models::open_router::{GrantLevel, MerchantTokenRequest};
    use common_enums::EntityType;

    // Looked up rather than read from the token, which carries only the user id. Display only, so
    // a failure here costs a name in the dashboard and nothing else.
    let email = match state.global_store.find_user_by_user_id(&user.user_id).await {
        Ok(found) => {
            // `Email` wraps a `Secret`, so it takes both traits to reach the string.
            use hyperswitch_masking::{ExposeInterface, PeekInterface};
            Some(
                domain::UserFromStorage::from(found)
                    .get_email()
                    .expose()
                    .peek()
                    .to_string(),
            )
        }
        Err(error) => {
            router_env::logger::warn!(
                ?error,
                "decision_engine_euclid: could not read email for SSO handoff"
            );
            None
        }
    };

    let profile_only = MerchantTokenRequest {
        merchant_id: profile_id.get_string_repr().to_string(),
        grant_level: None,
        grant_id: None,
        permissions: None,
        email: email.clone(),
    };

    let role_info =
        match crate::services::authorization::roles::RoleInfo::from_role_id_org_id_tenant_id(
            state,
            &user.role_id,
            &user.org_id,
            user.tenant_id.as_ref().unwrap_or(&state.tenant.tenant_id),
        )
        .await
        {
            Ok(role_info) => role_info,
            Err(error) => {
                // Never fails the handoff: the user is already authorized for this endpoint, so the
                // worst outcome of an unreadable role is the narrower session they had before.
                router_env::logger::warn!(
                ?error,
                "decision_engine_euclid: could not read role for SSO handoff, falling back to profile scope"
            );
                return profile_only;
            }
        };

    let (grant_level, grant_id) = match role_info.get_entity_type() {
        // A tenant role spans every org, which is not a node the tree can name. Treated as the org
        // the session is currently in, rather than granting more than one org at once.
        EntityType::Tenant | EntityType::Organization => (
            GrantLevel::Org,
            Some(user.org_id.get_string_repr().to_string()),
        ),
        EntityType::Merchant => (
            GrantLevel::Merchant,
            Some(user.merchant_id.get_string_repr().to_string()),
        ),
        EntityType::Profile => (GrantLevel::Profile, None),
    };

    // Read is implied — this endpoint sits behind ProfileRoutingRead, so reaching it at all means
    // the role has it. Only write has to be asked about.
    let mut permissions = vec!["routing:read".to_string()];
    if role_info.check_permission_exists(
        crate::services::authorization::permissions::Permission::ProfileRoutingWrite,
    ) {
        permissions.push("routing:write".to_string());
    }

    MerchantTokenRequest {
        merchant_id: profile_id.get_string_repr().to_string(),
        grant_level: Some(grant_level),
        grant_id,
        permissions: Some(permissions),
        email,
    }
}

/// Dashboard routing entry: reports the profile's routing source and, for a cut-over profile with a `target`, returns a one-time DE dashboard deep-link.
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn routing_entry(
    state: SessionState,
    platform: domain::Platform,
    user: crate::services::authentication::UserFromToken,
    profile_id: Option<common_utils::id_type::ProfileId>,
    target: Option<routing_types::DecisionEngineRoutingTarget>,
) -> RouterResponse<routing_types::RoutingEntryResponse> {
    let profile_id = profile_id.get_required_value("profile_id")?;

    let dimensions = dimension_state::Dimensions::new()
        .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
        .with_profile_id(profile_id.clone());

    // Flag-gated like every other cutover consumer, so the dashboard matches payment-path behavior.
    let is_cutover = is_decision_engine_routing_effective(&state, &dimensions).await;

    // Mint a fresh one-time code only when a card was clicked (`target`) on a cut-over profile.
    let redirect_url = match (is_cutover, target) {
        (true, Some(target)) => {
            let token_request = decision_engine_token_request(&state, &profile_id, user).await;
            let code = match helpers::mint_decision_engine_sso_code(&state, token_request.clone())
                .await
            {
                Ok(code) => code,
                // Provision the DE merchant only when it does not exist yet (DE returns 404), then retry once.
                Err(err)
                    if matches!(
                        err.current_context(),
                        errors::RoutingError::RoutingEventsError {
                            status_code: 404,
                            ..
                        }
                    ) =>
                {
                    // Provision with ancestry so a scope created on this path is grouped under its
                    // merchant, rather than landing in DE unattached. Falls back to the bare
                    // create if the profile cannot be read, since minting a code for a merchant
                    // that exists without ancestry still beats failing the dashboard entry.
                    let processor = platform.get_processor();
                    match core_utils::validate_and_get_business_profile(
                        state.store.as_ref(),
                        processor,
                        Some(&profile_id),
                    )
                    .await
                    .ok()
                    .flatten()
                    {
                        Some(profile) => {
                            let _ = helpers::sync_decision_engine_hierarchy(
                                &state,
                                processor.get_account(),
                                &profile,
                            )
                            .await;
                        }
                        None => {
                            let _ =
                                helpers::create_decision_engine_merchant(&state, &profile_id).await;
                        }
                    }
                    helpers::mint_decision_engine_sso_code(&state, token_request)
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)?
                }
                Err(err) => {
                    return Err(err)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to mint Decision Engine SSO code");
                }
            };
            Some(format!(
                "{}/{}?code={}",
                state.conf.open_router.dashboard_url,
                target.dashboard_path(),
                code
            ))
        }
        _ => None,
    };

    Ok(service_api::ApplicationResponse::Json(
        routing_types::RoutingEntryResponse {
            is_cutover,
            redirect_url,
        },
    ))
}

/// Effective cutover for this profile (per-profile source AND the global static flag).
#[cfg(feature = "v1")]
async fn is_profile_cutover_effective(
    state: &SessionState,
    platform: &domain::Platform,
    profile_id: common_utils::id_type::ProfileId,
) -> bool {
    let dimensions = dimension_state::Dimensions::new()
        .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
        .with_profile_id(profile_id);
    is_decision_engine_routing_effective(state, &dimensions).await
}

/// DE 4xx becomes an actionable InvalidRequestData with the DE's message; anything else stays a 500.
#[cfg(feature = "v1")]
fn map_de_write_error(
    error: error_stack::Report<errors::RoutingError>,
    printable: &'static str,
) -> error_stack::Report<errors::ApiErrorResponse> {
    let client_error_message = match error.current_context() {
        errors::RoutingError::RoutingEventsError {
            message,
            status_code,
        } if (400u16..500).contains(status_code) => Some(message.clone()),
        _ => None,
    };
    match client_error_message {
        Some(message) => error.change_context(errors::ApiErrorResponse::InvalidRequestData {
            message: format!("Decision Engine rejected the request: {message}"),
        }),
        None => error
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(printable),
    }
}

/// Converts to the DE shape; None for kinds the DE cannot host (3DS) or unconvertible programs.
#[cfg(feature = "v1")]
fn convert_euclid_algorithm_to_de_static(
    algorithm: routing_types::StaticRoutingAlgorithm,
) -> Option<StaticRoutingAlgorithm> {
    match algorithm {
        routing_types::StaticRoutingAlgorithm::Advanced(program) => match program.try_into() {
            Ok(internal_program) => Some(StaticRoutingAlgorithm::Advanced(internal_program)),
            Err(e) => {
                router_env::logger::error!(decision_engine_error = ?e, "decision_engine_euclid");
                None
            }
        },
        routing_types::StaticRoutingAlgorithm::Single(conn) => {
            Some(StaticRoutingAlgorithm::Single(Box::new((*conn).into())))
        }
        routing_types::StaticRoutingAlgorithm::Priority(connectors) => Some(
            StaticRoutingAlgorithm::Priority(connectors.into_iter().map(Into::into).collect()),
        ),
        routing_types::StaticRoutingAlgorithm::VolumeSplit(splits) => Some(
            StaticRoutingAlgorithm::VolumeSplit(splits.into_iter().map(Into::into).collect()),
        ),
        routing_types::StaticRoutingAlgorithm::ThreeDsDecisionRule(_) => {
            router_env::logger::error!(
                "decision_engine_euclid: ThreeDsDecisionRules are not yet implemented"
            );
            None
        }
    }
}

/// Mirrors the "already active" precondition on DE state; best-effort so a failed
/// listing never blocks the idempotent DE activation.
#[cfg(feature = "v1")]
async fn ensure_de_rule_not_already_active(
    state: &SessionState,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &enums::TransactionType,
    de_rule_id: &str,
) -> RouterResult<()> {
    let active_records =
        // Raw records: a rule HS cannot represent is still active on the DE.
        fetch_de_euclid_routing_records_raw(state, profile_id.get_string_repr().to_string(), true)
            .await
            .map_err(|error| {
                router_env::logger::warn!(
            ?error,
            "decision_engine_euclid: failed to list active DE rules for the already-active check"
        );
            })
            .unwrap_or_default();
    utils::when(
        active_records.iter().any(|record| {
            de_euclid_routing_record_algorithm_for(record).as_ref() == Some(transaction_type)
                && record.get("id").and_then(|id| id.as_str()) == Some(de_rule_id)
        }),
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Algorithm is already active".to_string(),
            })
        },
    )?;
    Ok(())
}

/// Looks for `algorithm_id` on the Decision Engine under a single profile.
///
/// `Ok(None)` means the profile is not cut over, or the DE simply has no such rule.
/// `Err` is reserved for the DE call itself failing, so callers can tell "no such rule"
/// apart from "could not ask".
#[cfg(feature = "v1")]
async fn find_de_record_in_profile(
    state: &SessionState,
    platform: &domain::Platform,
    business_profile: &domain::Profile,
    algorithm_id: &common_utils::id_type::RoutingId,
) -> Result<Option<RoutingAlgorithmRecord>, error_stack::Report<errors::RoutingError>> {
    if !is_profile_cutover_effective(state, platform, business_profile.get_id().clone()).await {
        return Ok(None);
    }

    let records = fetch_de_euclid_routing_records(
        state,
        business_profile.get_id().get_string_repr().to_string(),
        false,
    )
    .await
    .inspect_err(|error| {
        router_env::logger::warn!(
            ?error,
            profile_id = %business_profile.get_id().get_string_repr(),
            "decision_engine_euclid: failed to list DE rules while resolving a rule id"
        );
    })?;

    Ok(records
        .into_iter()
        .find(|record| &record.id == algorithm_id))
}

/// Finds a DE-only rule (no HS row) in the caller's profile, else the merchant's cut-over profiles.
#[cfg(feature = "v1")]
async fn find_de_record_for_algorithm(
    state: &SessionState,
    platform: &domain::Platform,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    algorithm_id: &common_utils::id_type::RoutingId,
) -> RouterResult<Option<(domain::Profile, RoutingAlgorithmRecord)>> {
    let db = state.store.as_ref();
    let processor = platform.get_processor();

    let candidate_profiles = match authentication_profile_id {
        Some(profile_id) => {
            core_utils::validate_and_get_business_profile(db, processor, Some(&profile_id))
                .await?
                .map(|profile| vec![profile])
                .unwrap_or_default()
        }
        None => db
            .list_profile_by_merchant_id(
                processor.get_key_store(),
                processor.get_account().get_id(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to list profiles while resolving a DE-only rule id")?,
    };

    // One profile's DE failure must not abort the scan (a later profile may hold the rule)
    // nor turn an unknown id into anything but a not-found; but if nothing matched and the
    // DE did fail, surface that instead of claiming the rule does not exist.
    let mut de_lookup_error = None;
    for business_profile in candidate_profiles {
        match find_de_record_in_profile(state, platform, &business_profile, algorithm_id).await {
            Ok(Some(record)) => return Ok(Some((business_profile, record))),
            Ok(None) => continue,
            Err(error) => {
                de_lookup_error = Some(error);
                continue;
            }
        }
    }

    match de_lookup_error {
        Some(error) => Err(map_de_write_error(
            error,
            "decision_engine_euclid: failed to list DE rules while resolving a rule id",
        )),
        None => Ok(None),
    }
}

/// Maps a DE rule record onto the same api shape a Hyperswitch-authored rule returns,
/// converting an advanced program back into the euclid AST.
#[cfg(feature = "v1")]
fn de_record_to_merchant_routing_algorithm(
    record: RoutingAlgorithmRecord,
) -> RouterResult<routing_types::MerchantRoutingAlgorithm> {
    let algorithm = match record.algorithm_data {
        StaticRoutingAlgorithm::Single(conn) => {
            routing_types::DeRoutableConnectorChoice::try_from(*conn)
                .map(|choice| {
                    routing_types::RoutingAlgorithmWrapper::Static(
                        routing_types::StaticRoutingAlgorithm::Single(Box::new(choice.into())),
                    )
                })
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to convert DE single connector to api shape")?
        }
        StaticRoutingAlgorithm::Priority(connectors) => connectors
            .into_iter()
            .map(|conn| routing_types::DeRoutableConnectorChoice::try_from(conn).map(Into::into))
            .collect::<Result<Vec<_>, _>>()
            .map(|choices| {
                routing_types::RoutingAlgorithmWrapper::Static(
                    routing_types::StaticRoutingAlgorithm::Priority(choices),
                )
            })
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to convert DE priority connectors to api shape")?,
        StaticRoutingAlgorithm::VolumeSplit(splits) => splits
            .into_iter()
            .map(|split| {
                routing_types::DeRoutableConnectorChoice::try_from(split.output).map(|choice| {
                    routing_types::ConnectorVolumeSplit {
                        connector: choice.into(),
                        split: split.split,
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|splits| {
                routing_types::RoutingAlgorithmWrapper::Static(
                    routing_types::StaticRoutingAlgorithm::VolumeSplit(splits),
                )
            })
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to convert DE volume split to api shape")?,
        StaticRoutingAlgorithm::Advanced(program) => {
            euclid::frontend::ast::Program::try_from(program)
                .map(|program| {
                    routing_types::RoutingAlgorithmWrapper::Static(
                        routing_types::StaticRoutingAlgorithm::Advanced(program),
                    )
                })
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to convert DE advanced algorithm to euclid AST")?
        }
    };

    Ok(routing_types::MerchantRoutingAlgorithm {
        id: record.id,
        profile_id: record.created_by,
        name: record.name,
        description: record.description.unwrap_or_default(),
        algorithm,
        created_at: record.created_at.assume_utc().unix_timestamp(),
        modified_at: record.modified_at.assume_utc().unix_timestamp(),
        algorithm_for: record.algorithm_for,
    })
}

pub enum TransactionData<'a> {
    Payment(PaymentsDslInput<'a>),
    #[cfg(feature = "payouts")]
    Payout(&'a payouts::PayoutData),
}

#[derive(Debug, Clone)]
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
        platform: &domain::Platform,
        profile_id: common_utils::id_type::ProfileId,
        transaction_type: enums::TransactionType,
    ) -> Self {
        let algorithm_id = common_utils::generate_routing_id_of_default_length();
        let timestamp = common_utils::date_time::now();
        let algo = RoutingAlgorithm {
            algorithm_id,
            profile_id,
            merchant_id: platform.get_provider().get_account().get_id().clone(),
            name: request.name.clone(),
            description: Some(request.description.clone()),
            kind: request.algorithm.get_kind().foreign_into(),
            algorithm_data: serde_json::json!(request.algorithm),
            created_at: timestamp,
            modified_at: timestamp,
            algorithm_for: transaction_type,
            decision_engine_routing_id: None,
            processor_merchant_id: Some(platform.get_processor().get_account().get_id().clone()),
            created_by: platform
                .get_initiator()
                .and_then(|initiator| initiator.to_created_by())
                .map(|created_by| created_by.to_string()),
        };
        Self(algo)
    }
    pub async fn fetch_routing_algo(
        processor_merchant_id: &common_utils::id_type::MerchantId,
        algorithm_id: &common_utils::id_type::RoutingId,
        db: &dyn StorageInterface,
    ) -> RouterResult<Self> {
        let routing_algo = db
            .find_routing_algorithm_by_algorithm_id_processor_merchant_id(
                algorithm_id,
                processor_merchant_id,
            )
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

    let result: Vec<routing_types::RoutingDictionaryRecord> = routing_metadata
        .into_iter()
        .map(ForeignInto::foreign_into)
        .collect::<Vec<_>>();

    // The merchant-level listing (no profile filter) stays Hyperswitch-only: the DE list
    // API is unpaginated, so merging it into a limit/offset page would break paging.
    // DE-backed rules are served by the profile-scoped listing below.
    #[cfg(feature = "v1")]
    let result =
        merge_de_routing_records(&state, &platform, result, profile_id_list, transaction_type)
            .await?;

    metrics::ROUTING_MERCHANT_DICTIONARY_RETRIEVE_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::RoutingKind::RoutingAlgorithm(result),
    ))
}

/// Merges DE-held rules into a profile-scoped listing, per profile and only for profiles
/// effectively cut over. Profiles served by Hyperswitch keep the HS records untouched.
#[cfg(feature = "v1")]
async fn merge_de_routing_records(
    state: &SessionState,
    platform: &domain::Platform,
    hs_result: Vec<routing_types::RoutingDictionaryRecord>,
    profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
    transaction_type: enums::TransactionType,
) -> RouterResult<Vec<routing_types::RoutingDictionaryRecord>> {
    let Some(profile_ids) = profile_id_list else {
        return Ok(hs_result);
    };

    let mut cutover_profiles = Vec::new();
    for profile_id in &profile_ids {
        if is_profile_cutover_effective(state, platform, profile_id.clone()).await {
            cutover_profiles.push(profile_id.clone());
        }
    }
    if cutover_profiles.is_empty() {
        return Ok(hs_result);
    }

    // Issued concurrently: the Decision Engine lists one profile per call, so a serial loop
    // would cost N round trips (each up to the 5s client timeout) in wall-clock time.
    // DE_TODO: a batch list endpoint on the Decision Engine would also cut the call count.
    let mut de_result: Vec<routing_types::RoutingDictionaryRecord> =
        futures::future::join_all(cutover_profiles.iter().map(|profile_id| async move {
            let list_request = ListRountingAlgorithmsRequest {
                created_by: profile_id.get_string_repr().to_string(),
            };
            list_de_euclid_routing_algorithms(state, list_request)
                .await
                .map_err(|e| {
                    router_env::logger::error!(decision_engine_error=?e, "decision_engine_euclid");
                })
                .ok() // Avoid throwing error if Decision Engine is not available or other errors
                .unwrap_or_default()
        }))
        .await
        .into_iter()
        .flatten()
        .collect();
    // filter de_result based on transaction type
    de_result.retain(|record| record.algorithm_for == Some(transaction_type));
    // append dynamic routing algorithms to de_result once (DE cannot represent them)
    de_result.extend(
        hs_result
            .iter()
            .filter(|record| record.kind == routing_types::RoutingAlgorithmKind::Dynamic)
            .cloned(),
    );
    compare_and_log_result(
        de_result.clone(),
        hs_result.clone(),
        "list_routing".to_string(),
        false,
    );

    let mut merged = build_list_routing_result(
        state,
        platform.clone(),
        &hs_result,
        &de_result,
        cutover_profiles.clone(),
    )
    .await?;
    // Profiles not cut over keep their Hyperswitch records verbatim.
    let cutover: HashSet<_> = cutover_profiles.into_iter().collect();
    merged.extend(
        hs_result
            .into_iter()
            .filter(|record| !cutover.contains(&record.profile_id)),
    );
    Ok(merged)
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
        let mut hs_result_for_profile: Vec<routing_types::RoutingDictionaryRecord> =
            hs_results.iter().filter(by_profile).cloned().collect();
        let business_profile = match core_utils::validate_and_get_business_profile(
            db,
            platform.get_processor(),
            Some(profile_id),
        )
        .await
        {
            Ok(Some(business_profile)) => business_profile,
            // A missing profile must not fail the listing; serve its HS records.
            Ok(None) | Err(_) => {
                router_env::logger::warn!(
                    profile_id = %profile_id.get_string_repr(),
                    "decision_engine_euclid: profile unavailable during routing list merge, serving HS records"
                );
                list_result.append(&mut hs_result_for_profile);
                continue;
            }
        };

        let dimensions = dimension_state::Dimensions::new()
            .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
            .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
            .with_profile_id(business_profile.get_id().clone());

        list_result.append(
            &mut select_routing_result(
                state,
                &dimensions,
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
    let processor = platform.get_processor();

    let business_profile =
        core_utils::validate_and_get_business_profile(db, processor, Some(&request.profile_id))
            .await?
            .get_required_value("Profile")?;
    let processor_merchant_id = processor.get_account().get_id();
    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;
    // Fetching disabled MCAs too: routing configs may reference MCAs that are
    // temporarily disabled — validation checks connector existence, not activity.
    let all_mcas = state
        .store
        .list_merchant_connector_accounts_without_encrypted_including_disabled_by_merchant_id_profile_id(
            processor_merchant_id,
            business_profile.get_id(),
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: processor_merchant_id.get_string_repr().to_owned(),
        })?;

    let name_mca_id_set = helpers::ConnectNameAndMCAIdForProfile(
        all_mcas
            .iter()
            .map(|mca| (&mca.connector_name, mca.get_id()))
            .collect(),
    );

    let name_set =
        helpers::ConnectNameForProfile(all_mcas.iter().map(|mca| &mca.connector_name).collect());

    let algorithm_helper = helpers::RoutingAlgorithmHelpers {
        name_mca_id_set,
        name_set,
        routing_algorithm: &request.algorithm,
    };

    algorithm_helper.validate_connectors_in_routing_config()?;

    let algo = RoutingAlgorithmUpdate::create_new_routing_algorithm(
        &request,
        &platform,
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
    let processor = platform.get_processor();
    let initiator = platform.get_initiator();

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

    let business_profile =
        core_utils::validate_and_get_business_profile(db, processor, Some(&profile_id))
            .await?
            .get_required_value("Profile")?;

    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;

    if algorithm.should_validate_connectors_in_routing_config() {
        helpers::validate_connectors_in_routing_config(
            &state,
            processor.get_key_store(),
            processor.get_account().get_id(),
            &profile_id,
            &algorithm,
        )
        .await?;
    }

    // 3DS decision rules cannot live on the DE, so they follow the HS path even under
    // cutover. Kind and transaction_type are independent request fields, so both must
    // exclude — otherwise a rule is written on one engine and read back from the other.
    let is_three_ds_rule = matches!(algorithm, EuclidAlgorithm::ThreeDsDecisionRule(_))
        || transaction_type == enums::TransactionType::ThreeDsAuthentication;
    let de_routing_effective = !is_three_ds_rule
        && is_profile_cutover_effective(&state, &platform, profile_id.clone()).await;

    // Provision the scope before writing into it: DE does not verify the scope exists, and
    // a rule attached to one with no merchant account routes fine but breaks the dashboard
    // handoff. Non-fatal for the dual-write path — merchants must be able to create rules
    // while DE is down — but required under cutover, where DE is the only destination.
    let de_scope_provisioned = match helpers::sync_decision_engine_hierarchy(
        &state,
        processor.get_account(),
        &business_profile,
    )
    .await
    {
        Ok(()) => true,
        Err(err) => {
            router_env::logger::warn!(
                decision_engine_error = ?err,
                profile_id = ?profile_id.get_string_repr(),
                "decision_engine_euclid: skipping rule dual-write, scope could not be provisioned"
            );
            if de_routing_effective {
                return Err(err)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "decision_engine_euclid: scope provisioning failed for a cut-over profile",
                    );
            }
            false
        }
    };

    let maybe_static_algorithm = convert_euclid_algorithm_to_de_static(algorithm.clone());

    let build_routing_rule = |static_algorithm: StaticRoutingAlgorithm| RoutingRule {
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

    if de_routing_effective {
        // DE-only write: hard-fail on DE errors; nothing is written to Hyperswitch.
        let static_algorithm =
            maybe_static_algorithm.ok_or(errors::ApiErrorResponse::InvalidRequestData {
                message: "This algorithm cannot be represented on the Decision Engine".to_string(),
            })?;
        let routing_rule = build_routing_rule(static_algorithm);

        let de_routing_id = create_de_euclid_routing_algo(&state, &routing_rule)
            .await
            .map_err(|error| map_de_write_error(error, "decision_engine_euclid: rule creation failed on the Decision Engine for a cut-over profile"))?;

        let timestamp = common_utils::date_time::now_unix_timestamp();
        let record = routing_types::RoutingDictionaryRecord {
            id: algorithm_id,
            profile_id,
            name: name.to_string(),
            kind: algorithm.get_kind(),
            description: description.clone(),
            created_at: timestamp,
            modified_at: timestamp,
            algorithm_for: Some(transaction_type.to_owned()),
            decision_engine_routing_id: Some(de_routing_id),
        };

        metrics::ROUTING_CREATE_SUCCESS_RESPONSE.add(1, &[]);
        return Ok(service_api::ApplicationResponse::Json(record));
    }

    let mut decision_engine_routing_id: Option<String> = None;

    // No scope on the DE means the dual-write would attach the rule to a merchant account
    // that does not exist; skip it and let the reconciliation report surface it as pending.
    if let Some(static_algorithm) = maybe_static_algorithm.filter(|_| de_scope_provisioned) {
        let routing_rule = build_routing_rule(static_algorithm);

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

    if decision_engine_routing_id.is_some() {
        router_env::logger::info!(routing_flow=?"create_euclid_routing_algorithm", is_equal=?"true", "decision_engine_euclid");
    } else {
        router_env::logger::info!(routing_flow=?"create_euclid_routing_algorithm", is_equal=?"false", "decision_engine_euclid");
    }

    let timestamp = common_utils::date_time::now();
    let algo = RoutingAlgorithm {
        algorithm_id: algorithm_id.clone(),
        profile_id,
        merchant_id: platform.get_provider().get_account().get_id().to_owned(),
        name: name.to_string(),
        description: Some(description.clone()),
        kind: algorithm.get_kind().foreign_into(),
        algorithm_data: serde_json::json!(algorithm),
        created_at: timestamp,
        modified_at: timestamp,
        algorithm_for: transaction_type.to_owned(),
        decision_engine_routing_id,
        processor_merchant_id: Some(processor.get_account().get_id().to_owned()),
        created_by: initiator
            .and_then(|initiator| initiator.to_created_by())
            .map(|created_by| created_by.to_string()),
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
    processor: domain::Processor,
    profile_id: common_utils::id_type::ProfileId,
    algorithm_id: common_utils::id_type::RoutingId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_LINK_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let routing_algorithm = RoutingAlgorithmUpdate::fetch_routing_algo(
        processor.get_account().get_id(),
        &algorithm_id,
        db,
    )
    .await?;

    utils::when(routing_algorithm.0.profile_id != profile_id, || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: "Profile Id is invalid for the routing config".to_string(),
        })
    })?;

    let business_profile =
        core_utils::validate_and_get_business_profile(db, &processor, Some(&profile_id))
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
            processor.get_key_store(),
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
    let processor = platform.get_processor().clone();

    let routing_algorithm = match db
        .find_routing_algorithm_by_algorithm_id_processor_merchant_id(
            &algorithm_id,
            processor.get_account().get_id(),
        )
        .await
    {
        Ok(routing_algorithm) => routing_algorithm,
        Err(error) => {
            // DE-only rule ids (e.g. created on the DE dashboard) activate directly on the DE.
            if let Some((business_profile, record)) = find_de_record_for_algorithm(
                &state,
                &platform,
                authentication_profile_id.clone(),
                &algorithm_id,
            )
            .await?
            {
                utils::when(record.algorithm_for != transaction_type, || {
                    Err(errors::ApiErrorResponse::PreconditionFailed {
                        message: format!(
                            "Cannot use {}'s routing algorithm for {} operation",
                            record.algorithm_for, transaction_type
                        ),
                    })
                })?;

                ensure_de_rule_not_already_active(
                    &state,
                    business_profile.get_id(),
                    &transaction_type,
                    record.id.get_string_repr(),
                )
                .await?;

                link_de_euclid_routing_algorithm(
                    &state,
                    ActivateRoutingConfigRequest {
                        created_by: business_profile.get_id().get_string_repr().to_string(),
                        routing_algorithm_id: record.id.get_string_repr().to_string(),
                    },
                )
                .await
                .map_err(|error| {
                    map_de_write_error(
                        error,
                        "decision_engine_euclid: rule activation failed on the Decision Engine",
                    )
                })?;

                metrics::ROUTING_LINK_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
                return Ok(service_api::ApplicationResponse::Json(
                    diesel_models::routing_algorithm::RoutingProfileMetadata::from(record)
                        .foreign_into(),
                ));
            }
            return Err(error).change_context(errors::ApiErrorResponse::ResourceIdNotFound);
        }
    };

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        &processor,
        Some(&routing_algorithm.profile_id),
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
                processor.get_key_store(),
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

            // 3DS decision rules stay HS-managed even under cutover; kind and
            // transaction_type must agree here and in create/unlink/list, or a rule is
            // written on one engine and read back from the other.
            let de_routing_effective = !matches!(
                routing_algorithm.kind,
                diesel_models::enums::RoutingAlgorithmKind::ThreeDsDecisionRule
            ) && transaction_type
                != enums::TransactionType::ThreeDsAuthentication
                && is_profile_cutover_effective(
                    &state,
                    &platform,
                    business_profile.get_id().clone(),
                )
                .await;

            if de_routing_effective {
                // Activate on the DE (hard-fail), leaving HS state untouched; rules the
                // DE doesn't know yet are created on it first.
                let de_rule_id = match routing_algorithm.decision_engine_routing_id.clone() {
                    Some(id) => id,
                    None => {
                        let profile_id_str =
                            business_profile.get_id().get_string_repr().to_string();
                        let existing_records =
                            fetch_de_euclid_routing_records(&state, profile_id_str.clone(), false)
                                .await
                                .map_err(|error| {
                                    map_de_write_error(
                                error,
                                "decision_engine_euclid: failed to list DE rules during activation",
                            )
                                })?;

                        if existing_records
                            .iter()
                            .any(|record| record.id == algorithm_id)
                        {
                            // Migrated rules carry the HS algorithm id on the DE side.
                            algorithm_id.get_string_repr().to_string()
                        } else {
                            let api_algorithm: routing_types::StaticRoutingAlgorithm =
                                routing_algorithm
                                    .algorithm_data
                                    .clone()
                                    .parse_value("StaticRoutingAlgorithm")
                                    .change_context(errors::ApiErrorResponse::InternalServerError)
                                    .attach_printable("unable to parse routing algorithm data")?;
                            let static_algorithm = convert_euclid_algorithm_to_de_static(
                                api_algorithm,
                            )
                            .ok_or(errors::ApiErrorResponse::InvalidRequestData {
                                message:
                                    "This algorithm cannot be represented on the Decision Engine"
                                        .to_string(),
                            })?;
                            let routing_rule = RoutingRule {
                                rule_id: Some(algorithm_id.get_string_repr().to_owned()),
                                name: routing_algorithm.name.clone(),
                                description: routing_algorithm.description.clone(),
                                created_by: profile_id_str,
                                algorithm: static_algorithm,
                                algorithm_for: routing_algorithm.algorithm_for.into(),
                                metadata: Some(RoutingMetadata {
                                    kind: routing_algorithm.kind,
                                }),
                            };
                            create_de_euclid_routing_algo(&state, &routing_rule)
                                .await
                                .map_err(|error| map_de_write_error(error, "decision_engine_euclid: rule creation failed on the Decision Engine during activation"))?
                        }
                    }
                };

                ensure_de_rule_not_already_active(
                    &state,
                    business_profile.get_id(),
                    &transaction_type,
                    &de_rule_id,
                )
                .await?;

                link_de_euclid_routing_algorithm(
                    &state,
                    ActivateRoutingConfigRequest {
                        created_by: business_profile.get_id().get_string_repr().to_string(),
                        routing_algorithm_id: de_rule_id,
                    },
                )
                .await
                .map_err(|error| {
                    map_de_write_error(
                        error,
                        "decision_engine_euclid: rule activation failed on the Decision Engine",
                    )
                })?;

                metrics::ROUTING_LINK_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
                return Ok(service_api::ApplicationResponse::Json(
                    routing_algorithm.foreign_into(),
                ));
            }

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
                processor.get_key_store(),
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
        processor.get_account().get_id(),
        business_profile.get_id(),
    )
    .await?;

    // redact routing cache on rule activation
    helpers::redact_routing_cache(
        &state,
        processor.get_account().get_id(),
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
    processor: domain::Processor,
    authentication_profile_id: Option<common_utils::id_type::ProfileId>,
    algorithm_id: common_utils::id_type::RoutingId,
) -> RouterResponse<routing_types::MerchantRoutingAlgorithm> {
    metrics::ROUTING_RETRIEVE_CONFIG.add(1, &[]);
    let db = state.store.as_ref();

    let routing_algorithm = RoutingAlgorithmUpdate::fetch_routing_algo(
        processor.get_account().get_id(),
        &algorithm_id,
        db,
    )
    .await?;
    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        &processor,
        Some(&routing_algorithm.0.profile_id),
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
    let processor = platform.get_processor().clone();

    let routing_algorithm = match db
        .find_routing_algorithm_by_algorithm_id_processor_merchant_id(
            &algorithm_id,
            processor.get_account().get_id(),
        )
        .await
    {
        Ok(routing_algorithm) => routing_algorithm,
        Err(error) => {
            // DE-only rules have no HS row; serve them from the DE.
            if let Some((_business_profile, record)) = find_de_record_for_algorithm(
                &state,
                &platform,
                authentication_profile_id.clone(),
                &algorithm_id,
            )
            .await?
            {
                let response = de_record_to_merchant_routing_algorithm(record)?;
                metrics::ROUTING_RETRIEVE_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
                return Ok(service_api::ApplicationResponse::Json(response));
            }
            return Err(error).to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound);
        }
    };

    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        &processor,
        Some(&routing_algorithm.profile_id),
    )
    .await?
    .get_required_value("Profile")
    .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?;

    core_utils::validate_profile_id_from_auth_layer(authentication_profile_id, &business_profile)?;

    // Cut-over first: the profile's rules live on the DE, which can edit a rule in place,
    // so an HS row for a migrated rule may be stale. Serve the DE copy when it has one and
    // keep the HS row as the fallback. A DE failure degrades to the HS row rather than
    // failing a read.
    match find_de_record_in_profile(&state, &platform, &business_profile, &algorithm_id).await {
        Ok(Some(record)) => {
            let response = de_record_to_merchant_routing_algorithm(record)?;
            metrics::ROUTING_RETRIEVE_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
            return Ok(service_api::ApplicationResponse::Json(response));
        }
        Ok(None) => (),
        Err(error) => router_env::logger::warn!(
            ?error,
            profile_id = %business_profile.get_id().get_string_repr(),
            "decision_engine_euclid: DE lookup failed on retrieve, serving the Hyperswitch row"
        ),
    }

    let response = routing_types::MerchantRoutingAlgorithm::foreign_try_from(routing_algorithm)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse routing algorithm")?;

    metrics::ROUTING_RETRIEVE_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn unlink_routing_config_under_profile(
    state: SessionState,
    processor: domain::Processor,
    profile_id: common_utils::id_type::ProfileId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::RoutingDictionaryRecord> {
    metrics::ROUTING_UNLINK_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let business_profile =
        core_utils::validate_and_get_business_profile(db, &processor, Some(&profile_id))
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
            processor.get_account().get_id(),
            &algorithm_id,
            db,
        )
        .await?;
        let response = record.0.foreign_into();
        admin::ProfileWrapper::new(business_profile)
            .update_profile_and_invalidate_routing_config_for_active_algorithm_id_update(
                db,
                key_manager_state,
                processor.get_key_store(),
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
    let processor = platform.get_processor().clone();

    let profile_id = request
        .profile_id
        .get_required_value("profile_id")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "profile_id",
        })
        .attach_printable("Profile_id not provided")?;

    let business_profile =
        core_utils::validate_and_get_business_profile(db, &processor, Some(&profile_id)).await?;

    match business_profile {
        Some(business_profile) => {
            core_utils::validate_profile_id_from_auth_layer(
                authentication_profile_id,
                &business_profile,
            )?;

            // 3DS decision rules stay HS-managed even under cutover.
            let de_routing_effective = transaction_type
                != enums::TransactionType::ThreeDsAuthentication
                && is_profile_cutover_effective(&state, &platform, profile_id.clone()).await;

            if de_routing_effective {
                // Deactivate on the DE (hard-fail). Raw records so an unrepresentable
                // active rule errors instead of reporting "already inactive".
                let profile_id_str = profile_id.get_string_repr().to_string();
                let raw_active_record = fetch_de_euclid_routing_records_raw(
                    &state,
                    profile_id_str.clone(),
                    true,
                )
                .await
                .map_err(|error| map_de_write_error(error, "decision_engine_euclid: failed to list active DE rules during deactivation"))?
                .into_iter()
                .find(|record| {
                    de_euclid_routing_record_algorithm_for(record) == Some(transaction_type)
                })
                .ok_or(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Algorithm is already inactive".to_string(),
                })?;
                let active_record = parse_de_euclid_routing_record(raw_active_record).ok_or(
                    errors::ApiErrorResponse::InvalidRequestData {
                        message: "The active rule on the Decision Engine cannot be managed through Hyperswitch; deactivate it from the Decision Engine dashboard".to_string(),
                    },
                )?;

                deactivate_de_euclid_routing_algorithm(
                    &state,
                    DeactivateRoutingConfigRequest {
                        created_by: profile_id_str,
                        routing_algorithm_id: active_record.id.get_string_repr().to_string(),
                    },
                )
                .await
                .map_err(|error| {
                    map_de_write_error(
                        error,
                        "decision_engine_euclid: rule deactivation failed on the Decision Engine",
                    )
                })?;

                metrics::ROUTING_UNLINK_CONFIG_SUCCESS_RESPONSE.add(1, &[]);
                return Ok(service_api::ApplicationResponse::Json(
                    diesel_models::routing_algorithm::RoutingProfileMetadata::from(active_record)
                        .foreign_into(),
                ));
            }
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
                        processor.get_key_store(),
                        business_profile.clone(),
                        routing_algorithm,
                        &transaction_type,
                    )
                    .await?;

                    // redact cgraph cache on rule activation
                    helpers::redact_cgraph_cache(
                        &state,
                        processor.get_account().get_id(),
                        business_profile.get_id(),
                    )
                    .await?;

                    // redact routing cache on rule activation
                    helpers::redact_routing_cache(
                        &state,
                        processor.get_account().get_id(),
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
    processor: domain::Processor,
    profile_id: common_utils::id_type::ProfileId,
    updated_list_of_connectors: Vec<routing_types::RoutableConnectorChoice>,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_UPDATE_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let profile = core_utils::validate_and_get_business_profile(db, &processor, Some(&profile_id))
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
            processor.get_key_store(),
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
    processor: domain::Processor,
    updated_config: Vec<routing_types::RoutableConnectorChoice>,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_UPDATE_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let default_config = helpers::get_merchant_default_config(
        db,
        processor.get_account().get_id().get_string_repr(),
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
        processor.get_account().get_id().get_string_repr(),
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
    processor: domain::Processor,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let profile = core_utils::validate_and_get_business_profile(db, &processor, Some(&profile_id))
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
    processor: domain::Processor,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
    metrics::ROUTING_RETRIEVE_DEFAULT_CONFIG.add(1, &[]);
    let db = state.store.as_ref();
    let id = profile_id
        .map(|profile_id| profile_id.get_string_repr().to_owned())
        .unwrap_or_else(|| {
            processor
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
    processor: domain::Processor,
    query_params: RoutingRetrieveQuery,
    profile_id: common_utils::id_type::ProfileId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::LinkedRoutingConfigRetrieveResponse> {
    metrics::ROUTING_RETRIEVE_LINK_CONFIG.add(1, &[]);
    let db = state.store.as_ref();

    let business_profile =
        core_utils::validate_and_get_business_profile(db, &processor, Some(&profile_id))
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
            platform.get_processor(),
            Some(&profile_id),
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

    // Prefetch the Decision Engine's active rules for every eligible profile at once. The
    // engine lists one profile per call, so issuing them inside the loop cost one serial
    // round trip per profile, each up to the 5s client timeout. A profile is eligible if
    // Hyperswitch has an active ref (the pre-existing behaviour) or it is cut over (where
    // the rule can live only on the engine). Absent from the map means "do not consult".
    let de_active_by_profile: HashMap<
        common_utils::id_type::ProfileId,
        Vec<routing_types::RoutingDictionaryRecord>,
    > = futures::future::join_all(business_profiles.iter().map(|business_profile| {
        let profile_id = business_profile.get_id().clone();
        // Parsed leniently only to decide eligibility; the loop below still parses
        // authoritatively and surfaces a malformed ref as an error.
        let has_hs_ref = match transaction_type {
            enums::TransactionType::Payment => &business_profile.routing_algorithm,
            #[cfg(feature = "payouts")]
            enums::TransactionType::Payout => &business_profile.payout_routing_algorithm,
            enums::TransactionType::ThreeDsAuthentication => {
                &business_profile.three_ds_decision_rule_algorithm
            }
        }
        .clone()
        .and_then(|val| {
            val.parse_value::<routing_types::RoutingAlgorithmRef>("RoutingAlgorithmRef")
                .ok()
        })
        .and_then(|routing_ref| routing_ref.algorithm_id)
        .is_some();
        let state = &state;
        let platform = &platform;
        async move {
            // 3DS rules never live on the DE.
            let de_routing_effective = transaction_type
                != enums::TransactionType::ThreeDsAuthentication
                && is_profile_cutover_effective(state, platform, profile_id.clone()).await;
            if !has_hs_ref && !de_routing_effective {
                return None;
            }
            let records = fetch_decision_engine_active_rules(state, &profile_id).await;
            Some((profile_id, records))
        }
    }))
    .await
    .into_iter()
    .flatten()
    .collect();

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

        let hs_records: Vec<routing_types::RoutingDictionaryRecord> = match routing_ref.algorithm_id
        {
            Some(algorithm_id) => {
                let record = db
                    .find_routing_algorithm_metadata_by_algorithm_id_profile_id(
                        &algorithm_id,
                        profile_id,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;
                vec![record.foreign_into()]
            }
            None => Vec::new(),
        };

        // Prefetched above: present only for profiles that have an HS ref or are cut over
        // (a cut-over profile can be active only on the DE, with no HS ref at all).
        if let Some(de_prefetched) = de_active_by_profile.get(profile_id).cloned() {
            let de_records = merge_decision_engine_active_rules(
                de_prefetched,
                &transaction_type,
                hs_records.clone(),
            );
            compare_and_log_result(
                de_records.clone(),
                hs_records.clone(),
                "list_active_routing".to_string(),
                false,
            );
            let dimensions = dimension_state::Dimensions::new()
                .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
                .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
                .with_profile_id(business_profile.get_id().clone());
            active_algorithms.append(
                &mut select_routing_result(
                    &state,
                    &dimensions,
                    &business_profile,
                    hs_records,
                    de_records,
                )
                .await,
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
    let de_records = fetch_decision_engine_active_rules(state, &profile_id).await;
    merge_decision_engine_active_rules(de_records, transaction_type, hs_records)
}

/// The network half of the active-rules lookup, split out so callers listing several
/// profiles can issue the per-profile calls concurrently instead of serially.
pub async fn fetch_decision_engine_active_rules(
    state: &SessionState,
    profile_id: &common_utils::id_type::ProfileId,
) -> Vec<routing_types::RoutingDictionaryRecord> {
    list_de_euclid_active_routing_algorithm(state, profile_id.get_string_repr().to_owned())
        .await
        .map_err(|e| {
            router_env::logger::error!(?e, "Failed to list DE Euclid active routing algorithm");
        })
        .ok() // Avoid throwing error if Decision Engine is not available or other errors thrown
        .unwrap_or_default()
}

/// The pure half: splice in the Hyperswitch dynamic algorithms the DE cannot represent and
/// keep only the requested transaction type.
pub fn merge_decision_engine_active_rules(
    mut de_records: Vec<routing_types::RoutingDictionaryRecord>,
    transaction_type: &enums::TransactionType,
    hs_records: Vec<routing_types::RoutingDictionaryRecord>,
) -> Vec<routing_types::RoutingDictionaryRecord> {
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
    processor: domain::Processor,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<Vec<routing_types::ProfileDefaultRoutingConfig>> {
    metrics::ROUTING_RETRIEVE_CONFIG_FOR_PROFILE.add(1, &[]);
    let db = state.store.as_ref();

    let all_profiles = db
        .list_profile_by_merchant_id(processor.get_key_store(), processor.get_account().get_id())
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
    processor: domain::Processor,
    updated_config: Vec<routing_types::RoutableConnectorChoice>,
    profile_id: common_utils::id_type::ProfileId,
    transaction_type: &enums::TransactionType,
) -> RouterResponse<routing_types::ProfileDefaultRoutingConfig> {
    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE.add(1, &[]);

    let db = state.store.as_ref();

    let business_profile =
        core_utils::validate_and_get_business_profile(db, &processor, Some(&profile_id))
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

    // Dual-write: also update business_profile.default_fallback_routing column
    let default_fallback_routing = Secret::from(
        updated_config
            .encode_to_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode updated config to value")?,
    );
    let profile_update = domain::ProfileUpdate::DefaultRoutingFallbackUpdate {
        default_fallback_routing: Some(default_fallback_routing),
    };
    db.update_profile_by_profile_id(
        processor.get_key_store(),
        business_profile.clone(),
        profile_update,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update default_fallback_routing in business profile")?;

    metrics::ROUTING_UPDATE_CONFIG_FOR_PROFILE_SUCCESS_RESPONSE.add(1, &[]);
    Ok(service_api::ApplicationResponse::Json(
        routing_types::ProfileDefaultRoutingConfig {
            profile_id: business_profile.get_id().to_owned(),
            connectors: updated_config,
        },
    ))
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
    let processor = platform.get_processor();

    let business_profile: domain::Profile =
        core_utils::validate_and_get_business_profile(db, processor, Some(&profile_id))
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
                &platform,
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
                processor.get_key_store().clone(),
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
    processor: domain::Processor,
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

    let business_profile: domain::Profile =
        core_utils::validate_and_get_business_profile(db, &processor, Some(&profile_id))
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
        processor.get_key_store(),
        business_profile.clone(),
        dynamic_routing_algo_ref.clone(),
    )
    .await?;

    Ok(service_api::ApplicationResponse::Json(routing_info))
}

#[cfg(feature = "v1")]
pub async fn retrieve_dynamic_routing_volume_split(
    state: SessionState,
    processor: domain::Processor,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResponse<routing_types::RoutingVolumeSplitResponse> {
    let db = state.store.as_ref();

    let business_profile: domain::Profile =
        core_utils::validate_and_get_business_profile(db, &processor, Some(&profile_id))
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

// check if this needs to stay
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
        processor_merchant_id: dynamic_routing_algo_to_update.processor_merchant_id,
        created_by: dynamic_routing_algo_to_update.created_by,
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
        processor_merchant_id: dynamic_routing_algo_to_update.processor_merchant_id,
        created_by: dynamic_routing_algo_to_update.created_by,
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
    let processor = platform.get_processor();

    let business_profile: domain::Profile =
        core_utils::validate_and_get_business_profile(db, processor, Some(&profile_id))
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
            processor.get_key_store().clone(),
            business_profile,
            algorithm,
            routing_types::DynamicRoutingType::ContractBasedRouting,
        )
        .await;
    }

    let config = config
        .get_required_value("ContractBasedRoutingConfig")
        .attach_printable("Failed to get ContractBasedRoutingConfig from request")?;

    let processor_merchant_id = processor.get_account().get_id().to_owned();
    let algorithm_id = common_utils::generate_routing_id_of_default_length();
    let timestamp = common_utils::date_time::now();

    let algo = RoutingAlgorithm {
        algorithm_id: algorithm_id.clone(),
        profile_id: profile_id.clone(),
        merchant_id: platform.get_provider().get_account().get_id().to_owned(),
        name: helpers::CONTRACT_BASED_DYNAMIC_ROUTING_ALGORITHM.to_string(),
        description: None,
        kind: diesel_models::enums::RoutingAlgorithmKind::Dynamic,
        algorithm_data: serde_json::json!(config),
        created_at: timestamp,
        modified_at: timestamp,
        algorithm_for: common_enums::TransactionType::Payment,
        decision_engine_routing_id: None,
        processor_merchant_id: Some(processor_merchant_id),
        created_by: platform
            .get_initiator()
            .and_then(|initiator| initiator.to_created_by())
            .map(|created_by| created_by.to_string()),
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
    if let Some(info_vec) = &config.label_info {
        helpers::validate_contract_based_label_info(
            db,
            processor.get_account().get_id(),
            &profile_id,
            info_vec,
        )
        .await?;
    }

    let record = db
        .insert_routing_algorithm(algo)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to insert record in routing algorithm table")?;

    helpers::update_business_profile_active_dynamic_algorithm_ref(
        db,
        processor.get_key_store(),
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
    processor: domain::Processor,
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
    if let Some(info_vec) = &request.label_info {
        helpers::validate_contract_based_label_info(
            db,
            processor.get_account().get_id(),
            &profile_id,
            info_vec,
        )
        .await?;
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
        processor_merchant_id: dynamic_routing_algo_to_update.processor_merchant_id,
        created_by: dynamic_routing_algo_to_update.created_by,
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
    async fn get_routable_connectors<F, D>(
        &self,
        state: &SessionState,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
        business_profile: &domain::Profile,
        payment_data: &mut D,
    ) -> RouterResult<RoutableConnectors>
    where
        F: Send + Clone,
        D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone;
}

pub struct StraightThroughAlgorithmTypeSingle(pub serde_json::Value);

#[async_trait]
impl GetRoutableConnectorsForChoice for StraightThroughAlgorithmTypeSingle {
    async fn get_routable_connectors<F, D>(
        &self,
        _state: &SessionState,
        _dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
        _business_profile: &domain::Profile,
        _payment_data: &mut D,
    ) -> RouterResult<RoutableConnectors>
    where
        F: Send + Clone,
        D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    {
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
    async fn get_routable_connectors<F, D>(
        &self,
        state: &SessionState,
        dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
        business_profile: &domain::Profile,
        payment_data: &mut D,
    ) -> RouterResult<RoutableConnectors>
    where
        F: Send + Clone,
        D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
    {
        let transaction_data = PaymentsDslInput::new(
            payment_data.get_setup_mandate(),
            payment_data.get_payment_attempt(),
            payment_data.get_payment_intent(),
            payment_data.get_payment_method_data(),
            payment_data.get_address(),
            payment_data.get_recurring_details(),
            payment_data.get_currency(),
        );
        let routing_algorithm_id = business_profile.get_payment_routing_algorithm_id()?;

        let (connectors, routing_approach) = payments_routing::perform_static_routing_v1(
            state,
            &business_profile.merchant_id,
            dimensions,
            routing_algorithm_id.as_ref(),
            business_profile,
            &TransactionData::Payment(transaction_data),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

        payment_data.set_routing_approach_in_attempt(routing_approach);

        Ok(RoutableConnectors(connectors))
    }
}

pub struct RoutableConnectors(Vec<routing_types::RoutableConnectorChoice>);

impl RoutableConnectors {
    pub fn filter_proxy_flow_supported_connectors(
        self,
        proxy_connector_filters: HashSet<String>,
    ) -> Self {
        let connectors = self
            .0
            .into_iter()
            .filter(|routable_connector_choice| {
                proxy_connector_filters.contains(&routable_connector_choice.connector.to_string())
            })
            .collect();
        Self(connectors)
    }

    pub async fn construct_dsl_and_perform_eligibility_analysis<F, D>(
        self,
        state: &SessionState,
        key_store: &domain::MerchantKeyStore,
        payment_data: &D,
        business_profile: &domain::Profile,
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

        let connectors = payments_routing::perform_eligibility_analysis_with_fallback(
            &state.clone(),
            key_store,
            routable_connector_choice,
            &TransactionData::Payment(payments_dsl_input),
            None,
            business_profile,
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

/// Clears the Decision Engine routing diff kill-switch counter for a profile, so the switch can
/// trip again after the profile is re-enabled for the Decision Engine.
pub async fn reset_decision_engine_diff_counter(
    state: SessionState,
    profile_id: common_utils::id_type::ProfileId,
) -> RouterResult<service_api::ApplicationResponse<()>> {
    reset_de_diff_counter(&state, &profile_id).await?;

    router_env::logger::info!(
        profile_id=?profile_id.get_string_repr(),
        "decision_engine_euclid: routing diff counter reset via api"
    );

    Ok(service_api::ApplicationResponse::StatusOk)
}

/// A merchant account and its key store, by id. Migration works from
/// `routing_algorithm.merchant_id` rather than an authenticated context, so it resolves its own.
async fn get_merchant_account(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
) -> RouterResult<(
    hyperswitch_domain_models::platform::MerchantKeyStore,
    domain::MerchantAccount,
)> {
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    Ok((key_store, merchant_account))
}

/// Migrates the named profiles' rules into the decision engine. One profile failing does not
/// stop the batch; a profile with no rules is reported as not attempted rather than as an error.
pub async fn migrate_rules_for_profiles(
    state: SessionState,
    request: routing_types::RuleMigrationRequest,
) -> RouterResult<routing_types::RuleMigrationResult> {
    let limit = request.validated_limit();
    let offset = request.offset.unwrap_or_default();

    // The merchant that owns each profile, which is what the business profile is resolved
    // under below. For a rule a platform wrote, that is its processor and not the provider the
    // call was made by — `find_rule_ids_for_profiles` resolves which.
    let merchant_of: HashMap<_, _> = state
        .store
        .find_rule_ids_for_profiles(&request.profile_ids)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?
        .into_iter()
        .map(|(profile_id, merchant_id, _, _)| (profile_id, merchant_id))
        .collect();

    let mut profiles = Vec::with_capacity(request.profile_ids.len());
    let mut totals = routing_types::RuleMigrationTotals {
        profiles: request.profile_ids.len(),
        ..Default::default()
    };

    // Once per merchant, before the loop: profiles cluster under merchants, and each read is two
    // store lookups plus decryption.
    let mut accounts: HashMap<common_utils::id_type::MerchantId, domain::Processor> =
        HashMap::new();
    let mut unreadable_merchants: HashSet<common_utils::id_type::MerchantId> = HashSet::new();

    for merchant_id in merchant_of.values().cloned().collect::<HashSet<_>>() {
        match get_merchant_account(&state, &merchant_id).await {
            Ok((key_store, merchant_account)) => {
                let platform = domain::Platform::new(
                    merchant_account.clone(),
                    key_store.clone(),
                    merchant_account,
                    key_store,
                    None,
                );
                accounts.insert(merchant_id, platform.get_processor().clone());
            }
            Err(err) => {
                router_env::logger::error!(
                    ?err,
                    merchant_id = ?merchant_id.get_string_repr(),
                    "could not read merchant account while migrating rules"
                );
                unreadable_merchants.insert(merchant_id);
            }
        }
    }

    for profile_id in request.profile_ids {
        let Some(merchant_id) = merchant_of.get(&profile_id).cloned() else {
            profiles.push(routing_types::RuleMigrationProfileResult {
                profile_id,
                merchant_id: None,
                success: vec![],
                skipped: vec![],
                errors: vec![],
                not_applicable: vec![],
                not_attempted: Some("no routing rules in Hyperswitch".to_string()),
            });
            totals.profiles_not_attempted += 1;
            continue;
        };

        let Some(processor) = accounts.get(&merchant_id).cloned() else {
            let reason = if unreadable_merchants.contains(&merchant_id) {
                "merchant account could not be read"
            } else {
                "merchant account not resolved"
            };
            profiles.push(routing_types::RuleMigrationProfileResult {
                profile_id,
                merchant_id: Some(merchant_id),
                success: vec![],
                skipped: vec![],
                errors: vec![],
                not_applicable: vec![],
                not_attempted: Some(reason.to_string()),
            });
            totals.profiles_not_attempted += 1;
            continue;
        };

        match migrate_rules_for_profile(&state, processor, profile_id.clone(), limit, offset).await
        {
            Ok(result) => {
                totals.rules_migrated += result.success.len();
                totals.rules_skipped += result.skipped.len();
                totals.rules_failed += result.errors.len();
                totals.rules_not_applicable += result.not_applicable.len();
                profiles.push(result);
            }
            Err(err) => {
                router_env::logger::error!(
                    ?err,
                    profile_id = ?profile_id.get_string_repr(),
                    "profile could not be migrated"
                );
                profiles.push(routing_types::RuleMigrationProfileResult {
                    profile_id,
                    merchant_id: Some(merchant_id),
                    success: vec![],
                    skipped: vec![],
                    errors: vec![],
                    not_applicable: vec![],
                    not_attempted: Some(err.current_context().to_string()),
                });
                totals.profiles_not_attempted += 1;
            }
        }
    }

    Ok(routing_types::RuleMigrationResult { profiles, totals })
}

/// Migrates one profile's rules. The batch entry point is `migrate_rules_for_profiles`.
async fn migrate_rules_for_profile(
    state: &SessionState,
    processor: domain::Processor,
    profile_id: common_utils::id_type::ProfileId,
    limit: u32,
    offset: u32,
) -> RouterResult<routing_types::RuleMigrationProfileResult> {
    use api_models::routing::StaticRoutingAlgorithm as EuclidAlgorithm;

    let state = state.clone();
    let db = state.store.as_ref();

    let business_profile =
        core_utils::validate_and_get_business_profile(db, &processor, Some(&profile_id))
            .await?
            .get_required_value("Profile")
            .change_context(errors::ApiErrorResponse::ProfileNotFound {
                id: profile_id.get_string_repr().to_owned(),
            })?;

    // Provision the scope, with its ancestry, before migrating any rule into it — otherwise the
    // rules route but the dashboard handoff returns "merchant not found".
    helpers::sync_decision_engine_hierarchy(&state, processor.get_account(), &business_profile)
        .await
        .attach_printable("Failed to provision decision engine scope before rule migration")?;

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
            i64::from(limit),
            i64::from(offset),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    // The decision engine keys rule create on the Hyperswitch algorithm id, so re-writing an
    // existing rule is a key conflict — without this a finished migration reads as total failure.
    // A read failure here is not fatal: the set stays empty and every rule is attempted.
    // Raw records: a rule shape Hyperswitch cannot represent is still present on the DE, and
    // reading it as absent would re-create it on every run.
    let already_in_decision_engine: HashSet<String> = match fetch_de_euclid_routing_records_raw(
        &state,
        profile_id.get_string_repr().to_string(),
        false,
    )
    .await
    {
        Ok(records) => de_euclid_routing_record_ids(&records).into_iter().collect(),
        Err(err) => {
            router_env::logger::warn!(
                decision_engine_error = ?err,
                profile_id = ?profile_id.get_string_repr(),
                "decision_engine_euclid: could not list existing rules, migrating without skipping"
            );
            HashSet::new()
        }
    };

    let mut response_list = Vec::new();
    let mut skipped_list = Vec::new();
    let mut error_list = Vec::new();
    let mut not_applicable_list = Vec::new();

    let mut push_error = |algorithm_id, msg: String| {
        error_list.push(RuleMigrationError {
            profile_id: profile_id.clone(),
            algorithm_id,
            error: msg,
        });
    };

    for routing_metadata in routing_metadatas {
        let algorithm_id = routing_metadata.algorithm_id.clone();

        // Held apart from `errors`: these kinds are not Euclid rules, so parsing one as an
        // algorithm fails every time and would keep a finished profile looking incomplete.
        let kind = routing_types::RoutingAlgorithmKind::foreign_from(routing_metadata.kind);
        if let Some(reason) = kind.rule_migration_exclusion() {
            not_applicable_list.push(routing_types::RuleMigrationNotApplicable {
                profile_id: profile_id.clone(),
                algorithm_id,
                kind,
                reason: reason.to_string(),
            });
            continue;
        }

        // Already migrated. The rule is left untouched — an insert cannot repair diverged
        // contents anyway. Linking is still attempted: a rule can arrive without being made
        // that profile's active one.
        if already_in_decision_engine.contains(algorithm_id.get_string_repr()) {
            let is_active_rule = active_payment_routing_ids.contains(&Some(algorithm_id.clone()));
            let mut linked_active_rule = false;
            if is_active_rule {
                match link_de_euclid_routing_algorithm(
                    &state,
                    ActivateRoutingConfigRequest {
                        created_by: profile_id.get_string_repr().to_string(),
                        routing_algorithm_id: algorithm_id.get_string_repr().to_string(),
                    },
                )
                .await
                {
                    Ok(()) => linked_active_rule = true,
                    Err(err) => {
                        router_env::logger::warn!(
                            decision_engine_error = ?err,
                            algorithm_id = ?algorithm_id,
                            "decision_engine_euclid: could not link an already-migrated active rule"
                        );
                    }
                }
            }
            skipped_list.push(routing_types::RuleMigrationSkipped {
                profile_id: profile_id.clone(),
                algorithm_id: algorithm_id.clone(),
                decision_engine_algorithm_id: algorithm_id.get_string_repr().to_string(),
                linked_active_rule,
            });
            continue;
        }

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
            // The decision engine requires a description; the column here is nullable, and a
            // null one is rejected outright rather than migrated.
            description: Some(algorithm.description.clone().unwrap_or_default()),
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

    Ok(routing_types::RuleMigrationProfileResult {
        profile_id,
        merchant_id: Some(processor.get_account().get_id().clone()),
        success: response_list,
        skipped: skipped_list,
        errors: error_list,
        not_applicable: not_applicable_list,
        not_attempted: None,
    })
}

/// Where every profile holding a routing rule stands in the migration, a page at a time. Each
/// source is read independently and its absence reported as unknown rather than assumed.
pub async fn routing_migration_status(
    state: SessionState,
    query: routing_types::RoutingMigrationStatusQuery,
) -> RouterResult<routing_types::RoutingMigrationStatusResponse> {
    let limit = query.validated_limit();
    let offset = query.offset.unwrap_or_default();

    let scopes = state
        .store
        .list_routing_scope_page(i64::from(limit), i64::from(offset))
        .await
        .to_not_found_response(errors::ApiErrorResponse::ResourceIdNotFound)?;

    // The page's rule ids in one query, so the comparison below costs nothing per profile.
    let page_profiles: Vec<_> = scopes
        .iter()
        .map(|(profile_id, _)| profile_id.clone())
        .collect();
    let mut hs_rule_ids: HashMap<common_utils::id_type::ProfileId, HashSet<String>> =
        HashMap::new();
    // Rules the migration is not expected to carry, counted apart so holding one never leaves a
    // finished profile reading as partial.
    let mut out_of_scope: HashMap<common_utils::id_type::ProfileId, i64> = HashMap::new();
    // The merchant that owns each profile. The page above is grouped by the rule's
    // `merchant_id`, which for a platform-written rule is the provider that made the call
    // rather than the merchant the profile hangs off — reporting that one names an account the
    // profile is not under, and is the merchant a migration would then fail to find it in.
    let mut owner_of: HashMap<common_utils::id_type::ProfileId, common_utils::id_type::MerchantId> =
        HashMap::new();
    match state.store.find_rule_ids_for_profiles(&page_profiles).await {
        Ok(rows) => {
            for (profile_id, owner_merchant_id, algorithm_id, kind) in rows {
                owner_of.insert(profile_id.clone(), owner_merchant_id);
                if routing_types::RoutingAlgorithmKind::foreign_from(kind)
                    .rule_migration_exclusion()
                    .is_some()
                {
                    *out_of_scope.entry(profile_id).or_default() += 1;
                    continue;
                }
                hs_rule_ids
                    .entry(profile_id)
                    .or_default()
                    .insert(algorithm_id.get_string_repr().to_string());
            }
        }
        Err(err) => {
            router_env::logger::warn!(?err, "could not read rule ids while reporting status");
        }
    }

    let has_more = scopes.len() == usize::try_from(limit).unwrap_or(usize::MAX);
    let mut profiles = Vec::with_capacity(scopes.len());
    let mut page_totals = routing_types::RoutingMigrationPageTotals {
        profiles: scopes.len(),
        ..Default::default()
    };

    for (profile_id, provider_merchant_id) in scopes {
        // Falls back to the grouping merchant only when the rule rows could not be read at all,
        // which is the same failure that leaves the rule counts empty.
        let merchant_id = owner_of
            .get(&profile_id)
            .cloned()
            .unwrap_or_else(|| provider_merchant_id.clone());

        // The listing already carries each rule's id, so comparing the sets is free.
        let de_rule_ids = match list_de_euclid_routing_algorithms(
            &state,
            ListRountingAlgorithmsRequest {
                created_by: profile_id.get_string_repr().to_string(),
            },
        )
        .await
        {
            Ok(records) => Some(
                records
                    .into_iter()
                    .map(|record| record.id.get_string_repr().to_string())
                    .collect::<HashSet<_>>(),
            ),
            Err(err) => {
                router_env::logger::warn!(
                    decision_engine_error = ?err,
                    profile_id = ?profile_id.get_string_repr(),
                    "decision_engine_euclid: could not read rules while reporting migration status"
                );
                None
            }
        };

        let rules_decision_engine = de_rule_ids
            .as_ref()
            .map(|ids| i64::try_from(ids.len()).unwrap_or(i64::MAX));

        let hs_ids = hs_rule_ids.get(&profile_id).cloned().unwrap_or_default();
        let rules_hyperswitch = i64::try_from(hs_ids.len()).unwrap_or(i64::MAX);
        let (mut missing_in_de, mut only_in_de) = (Vec::new(), Vec::new());
        if let Some(de_ids) = de_rule_ids.as_ref() {
            missing_in_de = hs_ids.difference(de_ids).cloned().collect();
            only_in_de = de_ids.difference(&hs_ids).cloned().collect();
            missing_in_de.sort();
            only_in_de.sort();
        }

        // The *configured* cut-over. `get_routing_result_source` would overlay Hyperswitch
        // routing when the diff counter is over threshold — right for a payment, wrong here: a
        // tripped kill switch would turn a runtime condition into a migration verdict.
        let dimensions = dimension_state::Dimensions::new()
            .with_processor_merchant_id(merchant_id.clone().into())
            .with_provider_merchant_id(
                hyperswitch_domain_models::platform::ProviderMerchantId::new(provider_merchant_id),
            )
            .with_profile_id(profile_id.clone());
        let routing_source = Some(
            dimensions
                .get_routing_result_source(
                    state.store.as_ref(),
                    state.superposition_service.as_ref(),
                    None,
                )
                .await,
        );

        let cut_over = matches!(
            routing_source,
            Some(routing_types::RoutingResultSource::DecisionEngine)
        );
        // `Partial` and `Diverged` both mean rules are missing; `Diverged` also means the counts
        // agree, so nothing that only counts will surface it.
        let state_of_profile = match rules_decision_engine {
            None => routing_types::RoutingMigrationState::Unknown,
            Some(0) if cut_over => routing_types::RoutingMigrationState::EnabledWithoutRules,
            Some(0) => routing_types::RoutingMigrationState::Pending,
            Some(count) if !missing_in_de.is_empty() && count == rules_hyperswitch => {
                routing_types::RoutingMigrationState::Diverged
            }
            Some(_) if !missing_in_de.is_empty() => routing_types::RoutingMigrationState::Partial,
            Some(_) if cut_over => routing_types::RoutingMigrationState::Enabled,
            Some(_) => routing_types::RoutingMigrationState::Migrated,
        };

        match state_of_profile {
            routing_types::RoutingMigrationState::Pending => page_totals.pending += 1,
            routing_types::RoutingMigrationState::Partial => page_totals.partial += 1,
            routing_types::RoutingMigrationState::Migrated => page_totals.migrated += 1,
            routing_types::RoutingMigrationState::Enabled => page_totals.enabled += 1,
            routing_types::RoutingMigrationState::EnabledWithoutRules => {
                page_totals.enabled_without_rules += 1
            }
            routing_types::RoutingMigrationState::Diverged => page_totals.diverged += 1,
            routing_types::RoutingMigrationState::Unknown => page_totals.unknown += 1,
        }

        profiles.push(routing_types::RoutingMigrationProfileStatus {
            rules_out_of_scope: out_of_scope.get(&profile_id).copied().unwrap_or_default(),
            profile_id,
            merchant_id,
            rules_hyperswitch,
            rules_decision_engine,
            rules_missing_in_decision_engine: missing_in_de,
            rules_only_in_decision_engine: only_in_de,
            routing_source,
            state: state_of_profile,
        });
    }

    Ok(routing_types::RoutingMigrationStatusResponse {
        profiles,
        limit,
        offset,
        has_more,
        page_totals,
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

pub fn transaction_type_from_payments_dsl(input: &PaymentsDslInput<'_>) -> enums::TransactionType {
    let txn_data = TransactionData::Payment(input.clone());
    enums::TransactionType::from(&txn_data)
}

pub fn log_connectors(stage: &str, connectors: &[routing::RoutableConnectorChoice]) {
    let names: Vec<String> = connectors.iter().map(|c| c.connector.to_string()).collect();

    router_env::logger::debug!(
        "euclid: connectors after {} = {{{}}}",
        stage,
        names.join(", ")
    );
}

pub fn convert_fallback_to_connector_routing_data(
    state: &SessionState,
    fallback: &[routing_types::RoutableConnectorChoice],
) -> RouterResult<Vec<api::ConnectorRoutingData>> {
    fallback
        .iter()
        .map(|choice| {
            let connector_name = choice.connector.to_string();

            let connector_data = api::ConnectorData::get_connector_by_name(
                &state.conf.connectors,
                &connector_name,
                api::GetToken::Connector,
                choice.merchant_connector_id.clone(),
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

            Ok(api::ConnectorRoutingData {
                connector_data,
                network: None,
                action_type: None,
            })
        })
        .collect()
}
