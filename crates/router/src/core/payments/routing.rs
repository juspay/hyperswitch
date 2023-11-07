mod transformers;

use std::{
    collections::hash_map,
    hash::{Hash, Hasher},
    sync::Arc,
};

use api_models::{
    admin as admin_api,
    enums::{self as api_enums, CountryAlpha2},
    routing::ConnectorSelection,
};
use common_utils::static_cache::StaticCache;
use diesel_models::enums as storage_enums;
use error_stack::{IntoReport, ResultExt};
use euclid::{
    backend::{self, inputs as dsl_inputs, EuclidBackend},
    dssa::graph::{self as euclid_graph, Memoization},
    enums as euclid_enums,
    frontend::ast,
};
use kgraph_utils::{
    mca as mca_graph,
    transformers::{IntoContext, IntoDirValue},
};
use masking::PeekInterface;
use rand::{
    distributions::{self, Distribution},
    SeedableRng,
};
use rustc_hash::FxHashMap;

#[cfg(not(feature = "business_profile_routing"))]
use crate::utils::StringExt;
use crate::{
    core::{
        errors as oss_errors, errors, payments as payments_oss, routing::helpers as routing_helpers,
    },
    logger,
    types::{
        api, api::routing as routing_types, domain, storage as oss_storage,
        transformers::ForeignInto,
    },
    utils::{OptionExt, ValueExt},
    AppState,
};

pub(super) enum CachedAlgorithm {
    Single(Box<routing_types::RoutableConnectorChoice>),
    Priority(Vec<routing_types::RoutableConnectorChoice>),
    VolumeSplit(Vec<routing_types::ConnectorVolumeSplit>),
    Advanced(backend::VirInterpreterBackend<ConnectorSelection>),
}

pub struct SessionFlowRoutingInput<'a> {
    pub state: &'a AppState,
    pub country: Option<CountryAlpha2>,
    pub key_store: &'a domain::MerchantKeyStore,
    pub merchant_account: &'a domain::MerchantAccount,
    pub payment_attempt: &'a oss_storage::PaymentAttempt,
    pub payment_intent: &'a oss_storage::PaymentIntent,
    pub chosen: Vec<api::SessionConnectorData>,
}

pub struct SessionRoutingPmTypeInput<'a> {
    state: &'a AppState,
    key_store: &'a domain::MerchantKeyStore,
    merchant_last_modified: i64,
    attempt_id: &'a str,
    routing_algorithm: &'a MerchantAccountRoutingAlgorithm,
    backend_input: dsl_inputs::BackendInput,
    allowed_connectors: FxHashMap<String, api::GetToken>,
    #[cfg(feature = "business_profile_routing")]
    profile_id: Option<String>,
}
static ROUTING_CACHE: StaticCache<CachedAlgorithm> = StaticCache::new();
static KGRAPH_CACHE: StaticCache<euclid_graph::KnowledgeGraph<'_>> = StaticCache::new();

type RoutingResult<O> = oss_errors::CustomResult<O, errors::RoutingError>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum MerchantAccountRoutingAlgorithm {
    V1(routing_types::RoutingAlgorithmRef),
}

impl Default for MerchantAccountRoutingAlgorithm {
    fn default() -> Self {
        Self::V1(routing_types::RoutingAlgorithmRef::default())
    }
}

pub fn make_dsl_input<F>(
    payment_data: &payments_oss::PaymentData<F>,
) -> RoutingResult<dsl_inputs::BackendInput>
where
    F: Clone,
{
    let mandate_data = dsl_inputs::MandateData {
        mandate_acceptance_type: payment_data
            .setup_mandate
            .as_ref()
            .and_then(|mandate_data| {
                mandate_data
                    .customer_acceptance
                    .clone()
                    .map(|cat| match cat.acceptance_type {
                        data_models::mandates::AcceptanceType::Online => {
                            euclid_enums::MandateAcceptanceType::Online
                        }
                        data_models::mandates::AcceptanceType::Offline => {
                            euclid_enums::MandateAcceptanceType::Offline
                        }
                    })
            }),
        mandate_type: payment_data
            .setup_mandate
            .as_ref()
            .and_then(|mandate_data| {
                mandate_data.mandate_type.clone().map(|mt| match mt {
                    data_models::mandates::MandateDataType::SingleUse(_) => {
                        euclid_enums::MandateType::SingleUse
                    }
                    data_models::mandates::MandateDataType::MultiUse(_) => {
                        euclid_enums::MandateType::MultiUse
                    }
                })
            }),
        payment_type: Some(payment_data.setup_mandate.clone().map_or_else(
            || euclid_enums::PaymentType::NonMandate,
            |_| euclid_enums::PaymentType::SetupMandate,
        )),
    };
    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: payment_data.payment_attempt.payment_method,
        payment_method_type: payment_data.payment_attempt.payment_method_type,
        card_network: payment_data
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                api::PaymentMethodData::Card(card) => card.card_network.clone(),

                _ => None,
            }),
    };

    let payment_input = dsl_inputs::PaymentInput {
        amount: payment_data.payment_intent.amount,
        card_bin: payment_data
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                api::PaymentMethodData::Card(card) => {
                    Some(card.card_number.peek().chars().take(6).collect())
                }
                _ => None,
            }),
        currency: payment_data.currency,
        authentication_type: payment_data.payment_attempt.authentication_type,
        capture_method: payment_data
            .payment_attempt
            .capture_method
            .and_then(|cm| cm.foreign_into()),
        business_country: payment_data
            .payment_intent
            .business_country
            .map(api_enums::Country::from_alpha2),
        billing_country: payment_data
            .address
            .billing
            .as_ref()
            .and_then(|bic| bic.address.as_ref())
            .and_then(|add| add.country)
            .map(api_enums::Country::from_alpha2),
        business_label: payment_data.payment_intent.business_label.clone(),
        setup_future_usage: payment_data.payment_intent.setup_future_usage,
    };

    let metadata = payment_data
        .payment_intent
        .metadata
        .clone()
        .map(|val| val.parse_value("routing_parameters"))
        .transpose()
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or_else(|err| {
            logger::error!(error=?err);
            None
        });

    Ok(dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: mandate_data,
    })
}

pub async fn perform_static_routing_v1<F: Clone>(
    state: &AppState,
    merchant_id: &str,
    algorithm_ref: routing_types::RoutingAlgorithmRef,
    payment_data: &mut payments_oss::PaymentData<F>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let algorithm_id = if let Some(id) = algorithm_ref.algorithm_id {
        id
    } else {
        let fallback_config =
            routing_helpers::get_merchant_default_config(&*state.clone().store, merchant_id)
                .await
                .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

        return Ok(fallback_config);
    };
    let key = ensure_algorithm_cached_v1(
        state,
        merchant_id,
        algorithm_ref.timestamp,
        &algorithm_id,
        #[cfg(feature = "business_profile_routing")]
        payment_data.payment_intent.profile_id.clone(),
    )
    .await?;
    let cached_algorithm: Arc<CachedAlgorithm> = ROUTING_CACHE
        .retrieve(&key)
        .into_report()
        .change_context(errors::RoutingError::CacheMiss)
        .attach_printable("Unable to retrieve cached routing algorithm even after refresh")?;

    Ok(match cached_algorithm.as_ref() {
        CachedAlgorithm::Single(conn) => vec![(**conn).clone()],

        CachedAlgorithm::Priority(plist) => plist.clone(),

        CachedAlgorithm::VolumeSplit(splits) => perform_volume_split(splits.to_vec(), None)
            .change_context(errors::RoutingError::ConnectorSelectionFailed)?,

        CachedAlgorithm::Advanced(interpreter) => {
            let backend_input = make_dsl_input(payment_data)?;

            execute_dsl_and_get_connector_v1(backend_input, interpreter)?
        }
    })
}

async fn ensure_algorithm_cached_v1(
    state: &AppState,
    merchant_id: &str,
    timestamp: i64,
    algorithm_id: &str,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<String> {
    #[cfg(feature = "business_profile_routing")]
    let key = {
        let profile_id = profile_id
            .clone()
            .get_required_value("profile_id")
            .change_context(errors::RoutingError::ProfileIdMissing)?;

        format!("routing_config_{merchant_id}_{profile_id}")
    };

    #[cfg(not(feature = "business_profile_routing"))]
    let key = format!("dsl_{merchant_id}");

    let present = ROUTING_CACHE
        .present(&key)
        .into_report()
        .change_context(errors::RoutingError::DslCachePoisoned)
        .attach_printable("Error checking presence of DSL")?;

    let expired = ROUTING_CACHE
        .expired(&key, timestamp)
        .into_report()
        .change_context(errors::RoutingError::DslCachePoisoned)
        .attach_printable("Error checking expiry of DSL in cache")?;

    if !present || expired {
        refresh_routing_cache_v1(
            state,
            key.clone(),
            algorithm_id,
            timestamp,
            #[cfg(feature = "business_profile_routing")]
            profile_id,
        )
        .await?;
    };

    Ok(key)
}

pub fn perform_straight_through_routing<F: Clone>(
    algorithm: &routing_types::StraightThroughAlgorithm,
    payment_data: &payments_oss::PaymentData<F>,
) -> RoutingResult<(Vec<routing_types::RoutableConnectorChoice>, bool)> {
    Ok(match algorithm {
        routing_types::StraightThroughAlgorithm::Single(conn) => (
            vec![(**conn).clone()],
            payment_data.creds_identifier.is_none(),
        ),

        routing_types::StraightThroughAlgorithm::Priority(conns) => (conns.clone(), true),

        routing_types::StraightThroughAlgorithm::VolumeSplit(splits) => (
            perform_volume_split(splits.to_vec(), None)
                .change_context(errors::RoutingError::ConnectorSelectionFailed)
                .attach_printable(
                    "Volume Split connector selection error in straight through routing",
                )?,
            true,
        ),
    })
}

fn execute_dsl_and_get_connector_v1(
    backend_input: dsl_inputs::BackendInput,
    interpreter: &backend::VirInterpreterBackend<ConnectorSelection>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let routing_output: routing_types::RoutingAlgorithm = interpreter
        .execute(backend_input)
        .map(|out| out.connector_selection.foreign_into())
        .into_report()
        .change_context(errors::RoutingError::DslExecutionError)?;

    Ok(match routing_output {
        routing_types::RoutingAlgorithm::Priority(plist) => plist,

        routing_types::RoutingAlgorithm::VolumeSplit(splits) => perform_volume_split(splits, None)
            .change_context(errors::RoutingError::DslFinalConnectorSelectionFailed)?,

        _ => Err(errors::RoutingError::DslIncorrectSelectionAlgorithm)
            .into_report()
            .attach_printable("Unsupported algorithm received as a result of static routing")?,
    })
}

pub async fn refresh_routing_cache_v1(
    state: &AppState,
    key: String,
    algorithm_id: &str,
    timestamp: i64,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<()> {
    #[cfg(feature = "business_profile_routing")]
    let algorithm = {
        let algorithm = state
            .store
            .find_routing_algorithm_by_profile_id_algorithm_id(
                &profile_id.unwrap_or_default(),
                algorithm_id,
            )
            .await
            .change_context(errors::RoutingError::DslMissingInDb)?;
        let algorithm: routing_types::RoutingAlgorithm = algorithm
            .algorithm_data
            .parse_value("RoutingAlgorithm")
            .change_context(errors::RoutingError::DslParsingError)?;
        algorithm
    };

    #[cfg(not(feature = "business_profile_routing"))]
    let algorithm = {
        let config = state
            .store
            .find_config_by_key(algorithm_id)
            .await
            .change_context(errors::RoutingError::DslMissingInDb)
            .attach_printable("DSL not found in DB")?;

        let algorithm: routing_types::RoutingAlgorithm = config
            .config
            .parse_struct("Program")
            .change_context(errors::RoutingError::DslParsingError)
            .attach_printable("Error parsing routing algorithm from configs")?;
        algorithm
    };
    let cached_algorithm = match algorithm {
        routing_types::RoutingAlgorithm::Single(conn) => CachedAlgorithm::Single(conn),
        routing_types::RoutingAlgorithm::Priority(plist) => CachedAlgorithm::Priority(plist),
        routing_types::RoutingAlgorithm::VolumeSplit(splits) => {
            CachedAlgorithm::VolumeSplit(splits)
        }
        routing_types::RoutingAlgorithm::Advanced(program) => {
            let interpreter = backend::VirInterpreterBackend::with_program(program)
                .into_report()
                .change_context(errors::RoutingError::DslBackendInitError)
                .attach_printable("Error initializing DSL interpreter backend")?;

            CachedAlgorithm::Advanced(interpreter)
        }
    };

    ROUTING_CACHE
        .save(key, cached_algorithm, timestamp)
        .into_report()
        .change_context(errors::RoutingError::DslCachePoisoned)
        .attach_printable("Error saving DSL to cache")?;

    Ok(())
}

pub fn perform_volume_split(
    mut splits: Vec<routing_types::ConnectorVolumeSplit>,
    rng_seed: Option<&str>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let weights: Vec<u8> = splits.iter().map(|sp| sp.split).collect();
    let weighted_index = distributions::WeightedIndex::new(weights)
        .into_report()
        .change_context(errors::RoutingError::VolumeSplitFailed)
        .attach_printable("Error creating weighted distribution for volume split")?;

    let idx = if let Some(seed) = rng_seed {
        let mut hasher = hash_map::DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(hash);
        weighted_index.sample(&mut rng)
    } else {
        let mut rng = rand::thread_rng();
        weighted_index.sample(&mut rng)
    };

    splits
        .get(idx)
        .ok_or(errors::RoutingError::VolumeSplitFailed)
        .into_report()
        .attach_printable("Volume split index lookup failed")?;

    // Panic Safety: We have performed a `get(idx)` operation just above which will
    // ensure that the index is always present, else throw an error.
    let removed = splits.remove(idx);
    splits.insert(0, removed);

    Ok(splits.into_iter().map(|sp| sp.connector).collect())
}

pub async fn get_merchant_kgraph<'a>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    merchant_last_modified: i64,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Arc<euclid_graph::KnowledgeGraph<'a>>> {
    #[cfg(feature = "business_profile_routing")]
    let key = {
        let profile_id = profile_id
            .clone()
            .get_required_value("profile_id")
            .change_context(errors::RoutingError::ProfileIdMissing)?;

        format!("kgraph_{}_{profile_id}", key_store.merchant_id)
    };

    #[cfg(not(feature = "business_profile_routing"))]
    let key = format!("kgraph_{}", key_store.merchant_id);

    let kgraph_present = KGRAPH_CACHE
        .present(&key)
        .into_report()
        .change_context(errors::RoutingError::KgraphCacheFailure)
        .attach_printable("when checking kgraph presence")?;

    let kgraph_expired = KGRAPH_CACHE
        .expired(&key, merchant_last_modified)
        .into_report()
        .change_context(errors::RoutingError::KgraphCacheFailure)
        .attach_printable("when checking kgraph expiry")?;

    if !kgraph_present || kgraph_expired {
        refresh_kgraph_cache(
            state,
            key_store,
            merchant_last_modified,
            key.clone(),
            #[cfg(feature = "business_profile_routing")]
            profile_id,
        )
        .await?;
    }

    let cached_kgraph = KGRAPH_CACHE
        .retrieve(&key)
        .into_report()
        .change_context(errors::RoutingError::CacheMiss)
        .attach_printable("when retrieving kgraph")?;

    Ok(cached_kgraph)
}

pub async fn refresh_kgraph_cache(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    timestamp: i64,
    key: String,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<()> {
    let mut merchant_connector_accounts = state
        .store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &key_store.merchant_id,
            false,
            key_store,
        )
        .await
        .change_context(errors::RoutingError::KgraphCacheRefreshFailed)?;

    merchant_connector_accounts
        .retain(|mca| mca.connector_type != storage_enums::ConnectorType::PaymentVas);

    #[cfg(feature = "business_profile_routing")]
    let merchant_connector_accounts = payments_oss::helpers::filter_mca_based_on_business_profile(
        merchant_connector_accounts,
        profile_id,
    );

    let api_mcas: Vec<admin_api::MerchantConnectorResponse> = merchant_connector_accounts
        .into_iter()
        .map(|acct| acct.try_into())
        .collect::<Result<_, _>>()
        .change_context(errors::RoutingError::KgraphCacheRefreshFailed)?;

    let kgraph = mca_graph::make_mca_graph(api_mcas)
        .into_report()
        .change_context(errors::RoutingError::KgraphCacheRefreshFailed)
        .attach_printable("when construction kgraph")?;

    KGRAPH_CACHE
        .save(key, kgraph, timestamp)
        .into_report()
        .change_context(errors::RoutingError::KgraphCacheRefreshFailed)
        .attach_printable("when saving kgraph to cache")?;

    Ok(())
}

async fn perform_kgraph_filtering(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    merchant_last_modified: i64,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    backend_input: dsl_inputs::BackendInput,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let context = euclid_graph::AnalysisContext::from_dir_values(
        backend_input
            .into_context()
            .into_report()
            .change_context(errors::RoutingError::KgraphAnalysisError)?,
    );
    let cached_kgraph = get_merchant_kgraph(
        state,
        key_store,
        merchant_last_modified,
        #[cfg(feature = "business_profile_routing")]
        profile_id,
    )
    .await?;

    let mut final_selection = Vec::<routing_types::RoutableConnectorChoice>::new();
    for choice in chosen {
        let routable_connector = choice.connector;
        let euclid_choice: ast::ConnectorChoice = choice.clone().foreign_into();
        let dir_val = euclid_choice
            .into_dir_value()
            .into_report()
            .change_context(errors::RoutingError::KgraphAnalysisError)?;
        let kgraph_eligible = cached_kgraph
            .check_value_validity(dir_val, &context, &mut Memoization::new())
            .into_report()
            .change_context(errors::RoutingError::KgraphAnalysisError)?;

        let filter_eligible =
            eligible_connectors.map_or(true, |list| list.contains(&routable_connector));

        if kgraph_eligible && filter_eligible {
            final_selection.push(choice);
        }
    }

    Ok(final_selection)
}

pub async fn perform_eligibility_analysis<F: Clone>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    merchant_last_modified: i64,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    payment_data: &payments_oss::PaymentData<F>,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let backend_input = make_dsl_input(payment_data)?;

    perform_kgraph_filtering(
        state,
        key_store,
        merchant_last_modified,
        chosen,
        backend_input,
        eligible_connectors,
        #[cfg(feature = "business_profile_routing")]
        profile_id,
    )
    .await
}

pub async fn perform_fallback_routing<F: Clone>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    merchant_last_modified: i64,
    payment_data: &payments_oss::PaymentData<F>,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let fallback_config =
        routing_helpers::get_merchant_default_config(&*state.store, &key_store.merchant_id)
            .await
            .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;
    let backend_input = make_dsl_input(payment_data)?;

    perform_kgraph_filtering(
        state,
        key_store,
        merchant_last_modified,
        fallback_config,
        backend_input,
        eligible_connectors,
        #[cfg(feature = "business_profile_routing")]
        profile_id,
    )
    .await
}

pub async fn perform_eligibility_analysis_with_fallback<F: Clone>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    merchant_last_modified: i64,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    payment_data: &payments_oss::PaymentData<F>,
    eligible_connectors: Option<Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let mut final_selection = perform_eligibility_analysis(
        state,
        key_store,
        merchant_last_modified,
        chosen,
        payment_data,
        eligible_connectors.as_ref(),
        #[cfg(feature = "business_profile_routing")]
        profile_id.clone(),
    )
    .await?;

    let fallback_selection = perform_fallback_routing(
        state,
        key_store,
        merchant_last_modified,
        payment_data,
        eligible_connectors.as_ref(),
        #[cfg(feature = "business_profile_routing")]
        profile_id,
    )
    .await;

    final_selection.append(
        &mut fallback_selection
            .unwrap_or_default()
            .iter()
            .filter(|&routable_connector_choice| {
                !final_selection.contains(routable_connector_choice)
            })
            .cloned()
            .collect::<Vec<_>>(),
    );

    let final_selected_connectors = final_selection
        .iter()
        .map(|item| item.connector)
        .collect::<Vec<_>>();
    logger::debug!(final_selected_connectors_for_routing=?final_selected_connectors, "List of final selected connectors for routing");

    Ok(final_selection)
}

pub async fn perform_session_flow_routing(
    session_input: SessionFlowRoutingInput<'_>,
) -> RoutingResult<FxHashMap<api_enums::PaymentMethodType, routing_types::SessionRoutingChoice>> {
    let mut pm_type_map: FxHashMap<api_enums::PaymentMethodType, FxHashMap<String, api::GetToken>> =
        FxHashMap::default();
    let merchant_last_modified = session_input
        .merchant_account
        .modified_at
        .assume_utc()
        .unix_timestamp();

    #[cfg(feature = "business_profile_routing")]
    let routing_algorithm: MerchantAccountRoutingAlgorithm = {
        let profile_id = session_input
            .payment_intent
            .profile_id
            .clone()
            .get_required_value("profile_id")
            .change_context(errors::RoutingError::ProfileIdMissing)?;

        let business_profile = session_input
            .state
            .store
            .find_business_profile_by_profile_id(&profile_id)
            .await
            .change_context(errors::RoutingError::ProfileNotFound)?;

        business_profile
            .routing_algorithm
            .clone()
            .map(|val| val.parse_value("MerchantAccountRoutingAlgorithm"))
            .transpose()
            .change_context(errors::RoutingError::InvalidRoutingAlgorithmStructure)?
            .unwrap_or_default()
    };

    #[cfg(not(feature = "business_profile_routing"))]
    let routing_algorithm: MerchantAccountRoutingAlgorithm = {
        session_input
            .merchant_account
            .routing_algorithm
            .clone()
            .map(|val| val.parse_value("MerchantAccountRoutingAlgorithm"))
            .transpose()
            .change_context(errors::RoutingError::InvalidRoutingAlgorithmStructure)?
            .unwrap_or_default()
    };

    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: None,
        payment_method_type: None,
        card_network: None,
    };

    let payment_input = dsl_inputs::PaymentInput {
        amount: session_input.payment_intent.amount,
        currency: session_input
            .payment_intent
            .currency
            .get_required_value("Currency")
            .change_context(errors::RoutingError::DslMissingRequiredField {
                field_name: "currency".to_string(),
            })?,
        authentication_type: session_input.payment_attempt.authentication_type,
        card_bin: None,
        capture_method: session_input
            .payment_attempt
            .capture_method
            .and_then(|cm| cm.foreign_into()),
        business_country: session_input
            .payment_intent
            .business_country
            .map(api_enums::Country::from_alpha2),
        billing_country: session_input
            .country
            .map(storage_enums::Country::from_alpha2),
        business_label: session_input.payment_intent.business_label.clone(),
        setup_future_usage: session_input.payment_intent.setup_future_usage,
    };

    let metadata = session_input
        .payment_intent
        .metadata
        .clone()
        .map(|val| val.parse_value("routing_parameters"))
        .transpose()
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or_else(|err| {
            logger::error!(?err);
            None
        });

    let mut backend_input = dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: dsl_inputs::MandateData {
            mandate_acceptance_type: None,
            mandate_type: None,
            payment_type: None,
        },
    };

    for connector_data in session_input.chosen.iter() {
        pm_type_map
            .entry(connector_data.payment_method_type)
            .or_default()
            .insert(
                connector_data.connector.connector_name.to_string(),
                connector_data.connector.get_token.clone(),
            );
    }

    let mut result: FxHashMap<api_enums::PaymentMethodType, routing_types::SessionRoutingChoice> =
        FxHashMap::default();

    for (pm_type, allowed_connectors) in pm_type_map {
        let euclid_pmt: euclid_enums::PaymentMethodType = pm_type;
        let euclid_pm: euclid_enums::PaymentMethod = euclid_pmt.into();

        backend_input.payment_method.payment_method = Some(euclid_pm);
        backend_input.payment_method.payment_method_type = Some(euclid_pmt);

        let session_pm_input = SessionRoutingPmTypeInput {
            state: session_input.state,
            key_store: session_input.key_store,
            merchant_last_modified,
            attempt_id: &session_input.payment_attempt.attempt_id,
            routing_algorithm: &routing_algorithm,
            backend_input: backend_input.clone(),
            allowed_connectors,
            #[cfg(feature = "business_profile_routing")]
            profile_id: session_input.payment_intent.clone().profile_id,
        };
        let maybe_choice = perform_session_routing_for_pm_type(session_pm_input).await?;

        // (connector, sub_label)
        if let Some(data) = maybe_choice {
            result.insert(
                pm_type,
                routing_types::SessionRoutingChoice {
                    connector: data.0,
                    #[cfg(not(feature = "connector_choice_mca_id"))]
                    sub_label: data.1,
                    payment_method_type: pm_type,
                },
            );
        }
    }

    Ok(result)
}

async fn perform_session_routing_for_pm_type(
    session_pm_input: SessionRoutingPmTypeInput<'_>,
) -> RoutingResult<Option<(api::ConnectorData, Option<String>)>> {
    let merchant_id = &session_pm_input.key_store.merchant_id;

    let chosen_connectors = match session_pm_input.routing_algorithm {
        MerchantAccountRoutingAlgorithm::V1(algorithm_ref) => {
            if let Some(ref algorithm_id) = algorithm_ref.algorithm_id {
                let key = ensure_algorithm_cached_v1(
                    &session_pm_input.state.clone(),
                    merchant_id,
                    algorithm_ref.timestamp,
                    algorithm_id,
                    #[cfg(feature = "business_profile_routing")]
                    session_pm_input.profile_id.clone(),
                )
                .await?;

                let cached_algorithm = ROUTING_CACHE
                    .retrieve(&key)
                    .into_report()
                    .change_context(errors::RoutingError::CacheMiss)
                    .attach_printable("unable to retrieve cached routing algorithm")?;

                match cached_algorithm.as_ref() {
                    CachedAlgorithm::Single(conn) => vec![(**conn).clone()],
                    CachedAlgorithm::Priority(plist) => plist.clone(),
                    CachedAlgorithm::VolumeSplit(splits) => {
                        perform_volume_split(splits.to_vec(), Some(session_pm_input.attempt_id))
                            .change_context(errors::RoutingError::ConnectorSelectionFailed)?
                    }
                    CachedAlgorithm::Advanced(interpreter) => execute_dsl_and_get_connector_v1(
                        session_pm_input.backend_input.clone(),
                        interpreter,
                    )?,
                }
            } else {
                routing_helpers::get_merchant_default_config(
                    &*session_pm_input.state.clone().store,
                    merchant_id,
                )
                .await
                .change_context(errors::RoutingError::FallbackConfigFetchFailed)?
            }
        }
    };

    let mut final_selection = perform_kgraph_filtering(
        &session_pm_input.state.clone(),
        session_pm_input.key_store,
        session_pm_input.merchant_last_modified,
        chosen_connectors,
        session_pm_input.backend_input.clone(),
        None,
        #[cfg(feature = "business_profile_routing")]
        session_pm_input.profile_id.clone(),
    )
    .await?;

    if final_selection.is_empty() {
        let fallback = routing_helpers::get_merchant_default_config(
            &*session_pm_input.state.clone().store,
            merchant_id,
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

        final_selection = perform_kgraph_filtering(
            &session_pm_input.state.clone(),
            session_pm_input.key_store,
            session_pm_input.merchant_last_modified,
            fallback,
            session_pm_input.backend_input,
            None,
            #[cfg(feature = "business_profile_routing")]
            session_pm_input.profile_id.clone(),
        )
        .await?;
    }

    let mut final_choice: Option<(api::ConnectorData, Option<String>)> = None;

    for selection in final_selection {
        let connector_name = selection.connector.to_string();
        if let Some(get_token) = session_pm_input.allowed_connectors.get(&connector_name) {
            let connector_data = api::ConnectorData::get_connector_by_name(
                &session_pm_input.state.clone().conf.connectors,
                &connector_name,
                get_token.clone(),
                #[cfg(feature = "connector_choice_mca_id")]
                selection.merchant_connector_id,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                None,
            )
            .change_context(errors::RoutingError::InvalidConnectorName(connector_name))?;
            #[cfg(not(feature = "connector_choice_mca_id"))]
            let sub_label = selection.sub_label;
            #[cfg(feature = "connector_choice_mca_id")]
            let sub_label = None;

            final_choice = Some((connector_data, sub_label));
            break;
        }
    }

    Ok(final_choice)
}
