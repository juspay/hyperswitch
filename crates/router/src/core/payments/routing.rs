mod transformers;

use std::{
    collections::{hash_map, HashMap},
    hash::{Hash, Hasher},
    str::FromStr,
    sync::Arc,
};

use api_models::{
    admin as admin_api,
    enums::{self as api_enums, CountryAlpha2},
    payments::Address,
    routing::ConnectorSelection,
};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use euclid::{
    backend::{self, inputs as dsl_inputs, EuclidBackend},
    dssa::graph::{self as euclid_graph, CgraphExt},
    enums as euclid_enums,
    frontend::{ast, dir as euclid_dir},
};
use kgraph_utils::{
    mca as mca_graph,
    transformers::{IntoContext, IntoDirValue},
    types::CountryCurrencyFilter,
};
use masking::PeekInterface;
use rand::{
    distributions::{self, Distribution},
    SeedableRng,
};
use rustc_hash::FxHashMap;
use storage_impl::redis::cache::{CGRAPH_CACHE, ROUTING_CACHE};

#[cfg(feature = "payouts")]
use crate::core::payouts;
#[cfg(not(feature = "business_profile_routing"))]
use crate::utils::StringExt;
use crate::{
    core::{
        errors, errors as oss_errors, payments as payments_oss,
        routing::{self, helpers as routing_helpers},
    },
    logger,
    types::{
        api::{self, routing as routing_types},
        domain, storage as oss_storage,
        transformers::{ForeignFrom, ForeignInto},
    },
    utils::{OptionExt, ValueExt},
    AppState,
};

pub enum CachedAlgorithm {
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
    attempt_id: &'a str,
    routing_algorithm: &'a MerchantAccountRoutingAlgorithm,
    backend_input: dsl_inputs::BackendInput,
    allowed_connectors: FxHashMap<String, api::GetToken>,
    #[cfg(any(
        feature = "business_profile_routing",
        feature = "profile_specific_fallback_routing"
    ))]
    profile_id: Option<String>,
}

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

#[cfg(feature = "payouts")]
pub fn make_dsl_input_for_payouts(
    payout_data: &payouts::PayoutData,
) -> RoutingResult<dsl_inputs::BackendInput> {
    let mandate = dsl_inputs::MandateData {
        mandate_acceptance_type: None,
        mandate_type: None,
        payment_type: None,
    };
    let metadata = payout_data
        .payouts
        .metadata
        .clone()
        .map(|val| val.parse_value("routing_parameters"))
        .transpose()
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payouts")
        .unwrap_or_else(|err| {
            logger::error!(error=?err);
            None
        });
    let payment = dsl_inputs::PaymentInput {
        amount: payout_data.payouts.amount,
        card_bin: None,
        currency: payout_data.payouts.destination_currency,
        authentication_type: None,
        capture_method: None,
        business_country: payout_data
            .payout_attempt
            .business_country
            .map(api_enums::Country::from_alpha2),
        billing_country: payout_data
            .billing_address
            .as_ref()
            .and_then(|bic| bic.country)
            .map(api_enums::Country::from_alpha2),
        business_label: payout_data.payout_attempt.business_label.clone(),
        setup_future_usage: None,
    };
    let payment_method = dsl_inputs::PaymentMethodInput {
        payment_method: Some(api_enums::PaymentMethod::foreign_from(
            payout_data.payouts.payout_type,
        )),
        payment_method_type: payout_data
            .payout_method_data
            .clone()
            .map(api_enums::PaymentMethodType::foreign_from),
        card_network: None,
    };
    Ok(dsl_inputs::BackendInput {
        mandate,
        metadata,
        payment,
        payment_method,
    })
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
                        hyperswitch_domain_models::mandates::AcceptanceType::Online => {
                            euclid_enums::MandateAcceptanceType::Online
                        }
                        hyperswitch_domain_models::mandates::AcceptanceType::Offline => {
                            euclid_enums::MandateAcceptanceType::Offline
                        }
                    })
            }),
        mandate_type: payment_data
            .setup_mandate
            .as_ref()
            .and_then(|mandate_data| {
                mandate_data.mandate_type.clone().map(|mt| match mt {
                    hyperswitch_domain_models::mandates::MandateDataType::SingleUse(_) => {
                        euclid_enums::MandateType::SingleUse
                    }
                    hyperswitch_domain_models::mandates::MandateDataType::MultiUse(_) => {
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
        amount: payment_data.payment_intent.amount.get_amount_as_i64(),
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
            .get_payment_method_billing()
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
    transaction_data: &routing::TransactionData<'_, F>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    #[cfg(any(
        feature = "profile_specific_fallback_routing",
        feature = "business_profile_routing"
    ))]
    let profile_id = match transaction_data {
        routing::TransactionData::Payment(payment_data) => payment_data
            .payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::RoutingError::ProfileIdMissing)?,
        #[cfg(feature = "payouts")]
        routing::TransactionData::Payout(payout_data) => &payout_data.payout_attempt.profile_id,
    };
    let algorithm_id = if let Some(id) = algorithm_ref.algorithm_id {
        id
    } else {
        let fallback_config = routing_helpers::get_merchant_default_config(
            &*state.clone().store,
            #[cfg(not(feature = "profile_specific_fallback_routing"))]
            merchant_id,
            #[cfg(feature = "profile_specific_fallback_routing")]
            profile_id,
            &api_enums::TransactionType::from(transaction_data),
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

        return Ok(fallback_config);
    };
    let cached_algorithm = ensure_algorithm_cached_v1(
        state,
        merchant_id,
        &algorithm_id,
        #[cfg(feature = "business_profile_routing")]
        Some(profile_id).cloned(),
        &api_enums::TransactionType::from(transaction_data),
    )
    .await?;

    Ok(match cached_algorithm.as_ref() {
        CachedAlgorithm::Single(conn) => vec![(**conn).clone()],

        CachedAlgorithm::Priority(plist) => plist.clone(),

        CachedAlgorithm::VolumeSplit(splits) => perform_volume_split(splits.to_vec(), None)
            .change_context(errors::RoutingError::ConnectorSelectionFailed)?,

        CachedAlgorithm::Advanced(interpreter) => {
            let backend_input = match transaction_data {
                routing::TransactionData::Payment(payment_data) => make_dsl_input(payment_data)?,
                #[cfg(feature = "payouts")]
                routing::TransactionData::Payout(payout_data) => {
                    make_dsl_input_for_payouts(payout_data)?
                }
            };

            execute_dsl_and_get_connector_v1(backend_input, interpreter)?
        }
    })
}

async fn ensure_algorithm_cached_v1(
    state: &AppState,
    merchant_id: &str,
    algorithm_id: &str,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Arc<CachedAlgorithm>> {
    #[cfg(feature = "business_profile_routing")]
    let key = {
        let profile_id = profile_id
            .clone()
            .get_required_value("profile_id")
            .change_context(errors::RoutingError::ProfileIdMissing)?;

        match transaction_type {
            api_enums::TransactionType::Payment => {
                format!("routing_config_{merchant_id}_{profile_id}")
            }
            #[cfg(feature = "payouts")]
            api_enums::TransactionType::Payout => {
                format!("routing_config_po_{merchant_id}_{profile_id}")
            }
        }
    };

    #[cfg(not(feature = "business_profile_routing"))]
    let key = match transaction_type {
        api_enums::TransactionType::Payment => {
            format!("dsl_{merchant_id}")
        }
        #[cfg(feature = "payouts")]
        api_enums::TransactionType::Payout => {
            format!("dsl_po_{merchant_id}")
        }
    };

    let cached_algorithm = ROUTING_CACHE
        .get_val::<Arc<CachedAlgorithm>>(key.as_str())
        .await;

    let algorithm = if let Some(algo) = cached_algorithm {
        algo
    } else {
        refresh_routing_cache_v1(
            state,
            key.clone(),
            algorithm_id,
            #[cfg(feature = "business_profile_routing")]
            profile_id,
        )
        .await?
    };

    Ok(algorithm)
}

pub fn perform_straight_through_routing(
    algorithm: &routing_types::StraightThroughAlgorithm,
    creds_identifier: Option<String>,
) -> RoutingResult<(Vec<routing_types::RoutableConnectorChoice>, bool)> {
    Ok(match algorithm {
        routing_types::StraightThroughAlgorithm::Single(conn) => {
            (vec![(**conn).clone()], creds_identifier.is_none())
        }

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
        .change_context(errors::RoutingError::DslExecutionError)?;

    Ok(match routing_output {
        routing_types::RoutingAlgorithm::Priority(plist) => plist,

        routing_types::RoutingAlgorithm::VolumeSplit(splits) => perform_volume_split(splits, None)
            .change_context(errors::RoutingError::DslFinalConnectorSelectionFailed)?,

        _ => Err(errors::RoutingError::DslIncorrectSelectionAlgorithm)
            .attach_printable("Unsupported algorithm received as a result of static routing")?,
    })
}

pub async fn refresh_routing_cache_v1(
    state: &AppState,
    key: String,
    algorithm_id: &str,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Arc<CachedAlgorithm>> {
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
                .change_context(errors::RoutingError::DslBackendInitError)
                .attach_printable("Error initializing DSL interpreter backend")?;

            CachedAlgorithm::Advanced(interpreter)
        }
    };

    let arc_cached_algorithm = Arc::new(cached_algorithm);

    ROUTING_CACHE.push(key, arc_cached_algorithm.clone()).await;

    Ok(arc_cached_algorithm)
}

pub fn perform_volume_split(
    mut splits: Vec<routing_types::ConnectorVolumeSplit>,
    rng_seed: Option<&str>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let weights: Vec<u8> = splits.iter().map(|sp| sp.split).collect();
    let weighted_index = distributions::WeightedIndex::new(weights)
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
        .attach_printable("Volume split index lookup failed")?;

    // Panic Safety: We have performed a `get(idx)` operation just above which will
    // ensure that the index is always present, else throw an error.
    let removed = splits.remove(idx);
    splits.insert(0, removed);

    Ok(splits.into_iter().map(|sp| sp.connector).collect())
}

pub async fn get_merchant_cgraph<'a>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Arc<hyperswitch_constraint_graph::ConstraintGraph<'a, euclid_dir::DirValue>>> {
    let merchant_id = &key_store.merchant_id;

    #[cfg(feature = "business_profile_routing")]
    let key = {
        let profile_id = profile_id
            .clone()
            .get_required_value("profile_id")
            .change_context(errors::RoutingError::ProfileIdMissing)?;
        match transaction_type {
            api_enums::TransactionType::Payment => format!("cgraph_{}_{}", merchant_id, profile_id),
            #[cfg(feature = "payouts")]
            api_enums::TransactionType::Payout => {
                format!("cgraph_po_{}_{}", merchant_id, profile_id)
            }
        }
    };

    #[cfg(not(feature = "business_profile_routing"))]
    let key = match transaction_type {
        api_enums::TransactionType::Payment => format!("kgraph_{}", merchant_id),
        #[cfg(feature = "payouts")]
        api_enums::TransactionType::Payout => format!("kgraph_po_{}", merchant_id),
    };

    let cached_cgraph = CGRAPH_CACHE
        .get_val::<Arc<hyperswitch_constraint_graph::ConstraintGraph<'_, euclid_dir::DirValue>>>(
            key.as_str(),
        )
        .await;

    let cgraph = if let Some(graph) = cached_cgraph {
        graph
    } else {
        refresh_cgraph_cache(
            state,
            key_store,
            key.clone(),
            #[cfg(feature = "business_profile_routing")]
            profile_id,
            transaction_type,
        )
        .await?
    };

    Ok(cgraph)
}

pub async fn refresh_cgraph_cache<'a>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    key: String,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Arc<hyperswitch_constraint_graph::ConstraintGraph<'a, euclid_dir::DirValue>>> {
    let mut merchant_connector_accounts = state
        .store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &key_store.merchant_id,
            false,
            key_store,
        )
        .await
        .change_context(errors::RoutingError::KgraphCacheRefreshFailed)?;

    match transaction_type {
        api_enums::TransactionType::Payment => {
            merchant_connector_accounts.retain(|mca| {
                mca.connector_type != storage_enums::ConnectorType::PaymentVas
                    && mca.connector_type != storage_enums::ConnectorType::PaymentMethodAuth
                    && mca.connector_type != storage_enums::ConnectorType::PayoutProcessor
                    && mca.connector_type != storage_enums::ConnectorType::AuthenticationProcessor
            });
        }
        #[cfg(feature = "payouts")]
        api_enums::TransactionType::Payout => {
            merchant_connector_accounts
                .retain(|mca| mca.connector_type == storage_enums::ConnectorType::PayoutProcessor);
        }
    };

    #[cfg(feature = "business_profile_routing")]
    let merchant_connector_accounts = payments_oss::helpers::filter_mca_based_on_business_profile(
        merchant_connector_accounts,
        profile_id,
    );

    let api_mcas = merchant_connector_accounts
        .into_iter()
        .map(admin_api::MerchantConnectorResponse::try_from)
        .collect::<Result<Vec<_>, _>>()
        .change_context(errors::RoutingError::KgraphCacheRefreshFailed)?;
    let connector_configs = state
        .conf
        .pm_filters
        .0
        .clone()
        .into_iter()
        .filter(|(key, _)| key != "default")
        .map(|(key, value)| {
            let key = api_enums::RoutableConnectors::from_str(&key)
                .map_err(|_| errors::RoutingError::InvalidConnectorName(key))?;

            Ok((key, value.foreign_into()))
        })
        .collect::<Result<HashMap<_, _>, errors::RoutingError>>()?;
    let default_configs = state
        .conf
        .pm_filters
        .0
        .get("default")
        .cloned()
        .map(ForeignFrom::foreign_from);
    let config_pm_filters = CountryCurrencyFilter {
        connector_configs,
        default_configs,
    };
    let cgraph = Arc::new(
        mca_graph::make_mca_graph(api_mcas, &config_pm_filters)
            .change_context(errors::RoutingError::KgraphCacheRefreshFailed)
            .attach_printable("when construction cgraph")?,
    );

    CGRAPH_CACHE.push(key, Arc::clone(&cgraph)).await;

    Ok(cgraph)
}

#[allow(clippy::too_many_arguments)]
async fn perform_cgraph_filtering(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    backend_input: dsl_inputs::BackendInput,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let context = euclid_graph::AnalysisContext::from_dir_values(
        backend_input
            .into_context()
            .change_context(errors::RoutingError::KgraphAnalysisError)?,
    );
    let cached_cgraph = get_merchant_cgraph(
        state,
        key_store,
        #[cfg(feature = "business_profile_routing")]
        profile_id,
        transaction_type,
    )
    .await?;

    let mut final_selection = Vec::<routing_types::RoutableConnectorChoice>::new();
    for choice in chosen {
        let routable_connector = choice.connector;
        let euclid_choice: ast::ConnectorChoice = choice.clone().foreign_into();
        let dir_val = euclid_choice
            .into_dir_value()
            .change_context(errors::RoutingError::KgraphAnalysisError)?;
        let cgraph_eligible = cached_cgraph
            .check_value_validity(
                dir_val,
                &context,
                &mut hyperswitch_constraint_graph::Memoization::new(),
                &mut hyperswitch_constraint_graph::CycleCheck::new(),
                None,
            )
            .change_context(errors::RoutingError::KgraphAnalysisError)?;

        let filter_eligible =
            eligible_connectors.map_or(true, |list| list.contains(&routable_connector));

        if cgraph_eligible && filter_eligible {
            final_selection.push(choice);
        }
    }

    Ok(final_selection)
}

pub async fn perform_eligibility_analysis<F: Clone>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    transaction_data: &routing::TransactionData<'_, F>,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let backend_input = match transaction_data {
        routing::TransactionData::Payment(payment_data) => make_dsl_input(payment_data)?,
        #[cfg(feature = "payouts")]
        routing::TransactionData::Payout(payout_data) => make_dsl_input_for_payouts(payout_data)?,
    };

    perform_cgraph_filtering(
        state,
        key_store,
        chosen,
        backend_input,
        eligible_connectors,
        #[cfg(feature = "business_profile_routing")]
        profile_id,
        &api_enums::TransactionType::from(transaction_data),
    )
    .await
}

pub async fn perform_fallback_routing<F: Clone>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    transaction_data: &routing::TransactionData<'_, F>,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let fallback_config = routing_helpers::get_merchant_default_config(
        &*state.store,
        #[cfg(not(feature = "profile_specific_fallback_routing"))]
        &key_store.merchant_id,
        #[cfg(feature = "profile_specific_fallback_routing")]
        match transaction_data {
            routing::TransactionData::Payment(payment_data) => payment_data
                .payment_intent
                .profile_id
                .as_ref()
                .get_required_value("profile_id")
                .change_context(errors::RoutingError::ProfileIdMissing)?,
            #[cfg(feature = "payouts")]
            routing::TransactionData::Payout(payout_data) => &payout_data.payout_attempt.profile_id,
        },
        &api_enums::TransactionType::from(transaction_data),
    )
    .await
    .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

    let backend_input = match transaction_data {
        routing::TransactionData::Payment(payment_data) => make_dsl_input(payment_data)?,
        #[cfg(feature = "payouts")]
        routing::TransactionData::Payout(payout_data) => make_dsl_input_for_payouts(payout_data)?,
    };

    perform_cgraph_filtering(
        state,
        key_store,
        fallback_config,
        backend_input,
        eligible_connectors,
        #[cfg(feature = "business_profile_routing")]
        profile_id,
        &api_enums::TransactionType::from(transaction_data),
    )
    .await
}

pub async fn perform_eligibility_analysis_with_fallback<F: Clone>(
    state: &AppState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    transaction_data: &routing::TransactionData<'_, F>,
    eligible_connectors: Option<Vec<api_enums::RoutableConnectors>>,
    #[cfg(feature = "business_profile_routing")] profile_id: Option<String>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let mut final_selection = perform_eligibility_analysis(
        state,
        key_store,
        chosen,
        transaction_data,
        eligible_connectors.as_ref(),
        #[cfg(feature = "business_profile_routing")]
        profile_id.clone(),
    )
    .await?;

    let fallback_selection = perform_fallback_routing(
        state,
        key_store,
        transaction_data,
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
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<FxHashMap<api_enums::PaymentMethodType, routing_types::SessionRoutingChoice>> {
    let mut pm_type_map: FxHashMap<api_enums::PaymentMethodType, FxHashMap<String, api::GetToken>> =
        FxHashMap::default();

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
        amount: session_input.payment_intent.amount.get_amount_as_i64(),
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
            attempt_id: &session_input.payment_attempt.attempt_id,
            routing_algorithm: &routing_algorithm,
            backend_input: backend_input.clone(),
            allowed_connectors,
            #[cfg(any(
                feature = "business_profile_routing",
                feature = "profile_specific_fallback_routing"
            ))]
            profile_id: session_input.payment_intent.profile_id.clone(),
        };
        let maybe_choice =
            perform_session_routing_for_pm_type(session_pm_input, transaction_type).await?;

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
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Option<(api::ConnectorData, Option<String>)>> {
    let merchant_id = &session_pm_input.key_store.merchant_id;

    let chosen_connectors = match session_pm_input.routing_algorithm {
        MerchantAccountRoutingAlgorithm::V1(algorithm_ref) => {
            if let Some(ref algorithm_id) = algorithm_ref.algorithm_id {
                let cached_algorithm = ensure_algorithm_cached_v1(
                    &session_pm_input.state.clone(),
                    merchant_id,
                    algorithm_id,
                    #[cfg(feature = "business_profile_routing")]
                    session_pm_input.profile_id.clone(),
                    transaction_type,
                )
                .await?;

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
                    #[cfg(not(feature = "profile_specific_fallback_routing"))]
                    merchant_id,
                    #[cfg(feature = "profile_specific_fallback_routing")]
                    {
                        session_pm_input
                            .profile_id
                            .as_ref()
                            .get_required_value("profile_id")
                            .change_context(errors::RoutingError::ProfileIdMissing)?
                    },
                    transaction_type,
                )
                .await
                .change_context(errors::RoutingError::FallbackConfigFetchFailed)?
            }
        }
    };

    let mut final_selection = perform_cgraph_filtering(
        &session_pm_input.state.clone(),
        session_pm_input.key_store,
        chosen_connectors,
        session_pm_input.backend_input.clone(),
        None,
        #[cfg(feature = "business_profile_routing")]
        session_pm_input.profile_id.clone(),
        transaction_type,
    )
    .await?;

    if final_selection.is_empty() {
        let fallback = routing_helpers::get_merchant_default_config(
            &*session_pm_input.state.clone().store,
            #[cfg(not(feature = "profile_specific_fallback_routing"))]
            merchant_id,
            #[cfg(feature = "profile_specific_fallback_routing")]
            {
                session_pm_input
                    .profile_id
                    .as_ref()
                    .get_required_value("profile_id")
                    .change_context(errors::RoutingError::ProfileIdMissing)?
            },
            transaction_type,
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

        final_selection = perform_cgraph_filtering(
            &session_pm_input.state.clone(),
            session_pm_input.key_store,
            fallback,
            session_pm_input.backend_input,
            None,
            #[cfg(feature = "business_profile_routing")]
            session_pm_input.profile_id.clone(),
            transaction_type,
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

pub fn make_dsl_input_for_surcharge(
    payment_attempt: &oss_storage::PaymentAttempt,
    payment_intent: &oss_storage::PaymentIntent,
    billing_address: Option<Address>,
) -> RoutingResult<dsl_inputs::BackendInput> {
    let mandate_data = dsl_inputs::MandateData {
        mandate_acceptance_type: None,
        mandate_type: None,
        payment_type: None,
    };
    let payment_input = dsl_inputs::PaymentInput {
        amount: payment_attempt.amount.get_amount_as_i64(),
        // currency is always populated in payment_attempt during payment create
        currency: payment_attempt
            .currency
            .get_required_value("currency")
            .change_context(errors::RoutingError::DslMissingRequiredField {
                field_name: "currency".to_string(),
            })?,
        authentication_type: payment_attempt.authentication_type,
        card_bin: None,
        capture_method: payment_attempt.capture_method,
        business_country: payment_intent
            .business_country
            .map(api_enums::Country::from_alpha2),
        billing_country: billing_address
            .and_then(|bic| bic.address)
            .and_then(|add| add.country)
            .map(api_enums::Country::from_alpha2),
        business_label: payment_intent.business_label.clone(),
        setup_future_usage: payment_intent.setup_future_usage,
    };
    let metadata = payment_intent
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
    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: None,
        payment_method_type: None,
        card_network: None,
    };
    let backend_input = dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: mandate_data,
    };
    Ok(backend_input)
}
