mod transformers;

use std::{
    collections::{hash_map, HashMap},
    hash::{Hash, Hasher},
    str::FromStr,
    sync::Arc,
};

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::routing as api_routing;
use api_models::{
    admin as admin_api,
    enums::{self as api_enums, CountryAlpha2},
    routing::ConnectorSelection,
};
#[cfg(feature = "dynamic_routing")]
use common_utils::ext_traits::AsyncExt;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use euclid::{
    backend::{self, inputs as dsl_inputs, EuclidBackend},
    dssa::graph::{self as euclid_graph, CgraphExt},
    enums as euclid_enums,
    frontend::{ast, dir as euclid_dir},
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use external_services::grpc_client::dynamic_routing::{
    contract_routing_client::{CalContractScoreResponse, ContractBasedDynamicRouting},
    success_rate_client::{CalSuccessRateResponse, SuccessBasedDynamicRouting},
};
use hyperswitch_domain_models::address::Address;
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
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use router_env::{instrument, tracing};
use rustc_hash::FxHashMap;
use storage_impl::redis::cache::{CacheKey, CGRAPH_CACHE, ROUTING_CACHE};

#[cfg(feature = "v2")]
use crate::core::admin;
#[cfg(feature = "payouts")]
use crate::core::payouts;
use crate::{
    core::{
        errors, errors as oss_errors, payments as payments_oss,
        routing::{self},
    },
    logger,
    types::{
        api::{self, routing as routing_types},
        domain, storage as oss_storage,
        transformers::{ForeignFrom, ForeignInto, ForeignTryFrom},
    },
    utils::{OptionExt, ValueExt},
    SessionState,
};

pub enum CachedAlgorithm {
    Single(Box<routing_types::RoutableConnectorChoice>),
    Priority(Vec<routing_types::RoutableConnectorChoice>),
    VolumeSplit(Vec<routing_types::ConnectorVolumeSplit>),
    Advanced(backend::VirInterpreterBackend<ConnectorSelection>),
}

pub struct SessionFlowRoutingInput<'a> {
    pub state: &'a SessionState,
    pub country: Option<CountryAlpha2>,
    pub key_store: &'a domain::MerchantKeyStore,
    pub merchant_account: &'a domain::MerchantAccount,
    pub payment_attempt: &'a oss_storage::PaymentAttempt,
    pub payment_intent: &'a oss_storage::PaymentIntent,
    pub chosen: Vec<api::SessionConnectorData>,
}

pub struct SessionRoutingPmTypeInput<'a> {
    state: &'a SessionState,
    key_store: &'a domain::MerchantKeyStore,
    attempt_id: &'a str,
    routing_algorithm: &'a MerchantAccountRoutingAlgorithm,
    backend_input: dsl_inputs::BackendInput,
    allowed_connectors: FxHashMap<String, api::GetToken>,
    profile_id: &'a common_utils::id_type::ProfileId,
}

type RoutingResult<O> = oss_errors::CustomResult<O, errors::RoutingError>;

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum MerchantAccountRoutingAlgorithm {
    V1(routing_types::RoutingAlgorithmRef),
}

#[cfg(feature = "v1")]
impl Default for MerchantAccountRoutingAlgorithm {
    fn default() -> Self {
        Self::V1(routing_types::RoutingAlgorithmRef::default())
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum MerchantAccountRoutingAlgorithm {
    V1(Option<common_utils::id_type::RoutingId>),
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
        .unwrap_or(None);
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
        payment_method: payout_data
            .payouts
            .payout_type
            .map(api_enums::PaymentMethod::foreign_from),
        payment_method_type: payout_data
            .payout_method_data
            .as_ref()
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

#[cfg(feature = "v2")]
pub fn make_dsl_input(
    payments_dsl_input: &routing::PaymentsDslInput<'_>,
) -> RoutingResult<dsl_inputs::BackendInput> {
    let mandate_data = dsl_inputs::MandateData {
        mandate_acceptance_type: payments_dsl_input.setup_mandate.as_ref().and_then(
            |mandate_data| {
                mandate_data
                    .customer_acceptance
                    .as_ref()
                    .map(|customer_accept| match customer_accept.acceptance_type {
                        hyperswitch_domain_models::mandates::AcceptanceType::Online => {
                            euclid_enums::MandateAcceptanceType::Online
                        }
                        hyperswitch_domain_models::mandates::AcceptanceType::Offline => {
                            euclid_enums::MandateAcceptanceType::Offline
                        }
                    })
            },
        ),
        mandate_type: payments_dsl_input
            .setup_mandate
            .as_ref()
            .and_then(|mandate_data| {
                mandate_data
                    .mandate_type
                    .clone()
                    .map(|mandate_type| match mandate_type {
                        hyperswitch_domain_models::mandates::MandateDataType::SingleUse(_) => {
                            euclid_enums::MandateType::SingleUse
                        }
                        hyperswitch_domain_models::mandates::MandateDataType::MultiUse(_) => {
                            euclid_enums::MandateType::MultiUse
                        }
                    })
            }),
        payment_type: Some(
            if payments_dsl_input
                .recurring_details
                .as_ref()
                .is_some_and(|data| {
                    matches!(
                        data,
                        api_models::mandates::RecurringDetails::ProcessorPaymentToken(_)
                    )
                })
            {
                euclid_enums::PaymentType::PptMandate
            } else {
                payments_dsl_input.setup_mandate.map_or_else(
                    || euclid_enums::PaymentType::NonMandate,
                    |_| euclid_enums::PaymentType::SetupMandate,
                )
            },
        ),
    };
    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: Some(payments_dsl_input.payment_attempt.payment_method_type),
        payment_method_type: Some(payments_dsl_input.payment_attempt.payment_method_subtype),
        card_network: payments_dsl_input
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => card.card_network.clone(),

                _ => None,
            }),
    };

    let payment_input = dsl_inputs::PaymentInput {
        amount: payments_dsl_input
            .payment_attempt
            .amount_details
            .get_net_amount(),
        card_bin: payments_dsl_input.payment_method_data.as_ref().and_then(
            |pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => Some(card.card_number.get_card_isin()),
                _ => None,
            },
        ),
        currency: payments_dsl_input.currency,
        authentication_type: Some(payments_dsl_input.payment_attempt.authentication_type),
        capture_method: Some(payments_dsl_input.payment_intent.capture_method),
        business_country: None,
        billing_country: payments_dsl_input
            .address
            .get_payment_method_billing()
            .and_then(|billing_address| billing_address.address.as_ref())
            .and_then(|address_details| address_details.country)
            .map(api_enums::Country::from_alpha2),
        business_label: None,
        setup_future_usage: Some(payments_dsl_input.payment_intent.setup_future_usage),
    };

    let metadata = payments_dsl_input
        .payment_intent
        .metadata
        .clone()
        .map(|value| value.parse_value("routing_parameters"))
        .transpose()
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or(None);

    Ok(dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: mandate_data,
    })
}

#[cfg(feature = "v1")]
pub fn make_dsl_input(
    payments_dsl_input: &routing::PaymentsDslInput<'_>,
) -> RoutingResult<dsl_inputs::BackendInput> {
    let mandate_data = dsl_inputs::MandateData {
        mandate_acceptance_type: payments_dsl_input.setup_mandate.as_ref().and_then(
            |mandate_data| {
                mandate_data
                    .customer_acceptance
                    .as_ref()
                    .map(|cat| match cat.acceptance_type {
                        hyperswitch_domain_models::mandates::AcceptanceType::Online => {
                            euclid_enums::MandateAcceptanceType::Online
                        }
                        hyperswitch_domain_models::mandates::AcceptanceType::Offline => {
                            euclid_enums::MandateAcceptanceType::Offline
                        }
                    })
            },
        ),
        mandate_type: payments_dsl_input
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
        payment_type: Some(
            if payments_dsl_input
                .recurring_details
                .as_ref()
                .is_some_and(|data| {
                    matches!(
                        data,
                        api_models::mandates::RecurringDetails::ProcessorPaymentToken(_)
                    )
                })
            {
                euclid_enums::PaymentType::PptMandate
            } else {
                payments_dsl_input.setup_mandate.map_or_else(
                    || euclid_enums::PaymentType::NonMandate,
                    |_| euclid_enums::PaymentType::SetupMandate,
                )
            },
        ),
    };
    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: payments_dsl_input.payment_attempt.payment_method,
        payment_method_type: payments_dsl_input.payment_attempt.payment_method_type,
        card_network: payments_dsl_input
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => card.card_network.clone(),

                _ => None,
            }),
    };

    let payment_input = dsl_inputs::PaymentInput {
        amount: payments_dsl_input.payment_attempt.get_total_amount(),
        card_bin: payments_dsl_input.payment_method_data.as_ref().and_then(
            |pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => {
                    Some(card.card_number.peek().chars().take(6).collect())
                }
                _ => None,
            },
        ),
        currency: payments_dsl_input.currency,
        authentication_type: payments_dsl_input.payment_attempt.authentication_type,
        capture_method: payments_dsl_input
            .payment_attempt
            .capture_method
            .and_then(|cm| cm.foreign_into()),
        business_country: payments_dsl_input
            .payment_intent
            .business_country
            .map(api_enums::Country::from_alpha2),
        billing_country: payments_dsl_input
            .address
            .get_payment_method_billing()
            .and_then(|bic| bic.address.as_ref())
            .and_then(|add| add.country)
            .map(api_enums::Country::from_alpha2),
        business_label: payments_dsl_input.payment_intent.business_label.clone(),
        setup_future_usage: payments_dsl_input.payment_intent.setup_future_usage,
    };

    let metadata = payments_dsl_input
        .payment_intent
        .metadata
        .clone()
        .map(|val| val.parse_value("routing_parameters"))
        .transpose()
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or(None);

    Ok(dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: mandate_data,
    })
}

pub async fn perform_static_routing_v1(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    algorithm_id: Option<&common_utils::id_type::RoutingId>,
    business_profile: &domain::Profile,
    transaction_data: &routing::TransactionData<'_>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let algorithm_id = if let Some(id) = algorithm_id {
        id
    } else {
        #[cfg(feature = "v1")]
        let fallback_config = routing::helpers::get_merchant_default_config(
            &*state.clone().store,
            business_profile.get_id().get_string_repr(),
            &api_enums::TransactionType::from(transaction_data),
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;
        #[cfg(feature = "v2")]
        let fallback_config = admin::ProfileWrapper::new(business_profile.clone())
            .get_default_fallback_list_of_connector_under_profile()
            .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

        return Ok(fallback_config);
    };
    let cached_algorithm = ensure_algorithm_cached_v1(
        state,
        merchant_id,
        algorithm_id,
        business_profile.get_id(),
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
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    algorithm_id: &common_utils::id_type::RoutingId,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Arc<CachedAlgorithm>> {
    let key = {
        match transaction_type {
            common_enums::TransactionType::Payment => {
                format!(
                    "routing_config_{}_{}",
                    merchant_id.get_string_repr(),
                    profile_id.get_string_repr(),
                )
            }
            #[cfg(feature = "payouts")]
            common_enums::TransactionType::Payout => {
                format!(
                    "routing_config_po_{}_{}",
                    merchant_id.get_string_repr(),
                    profile_id.get_string_repr()
                )
            }
        }
    };

    let cached_algorithm = ROUTING_CACHE
        .get_val::<Arc<CachedAlgorithm>>(CacheKey {
            key: key.clone(),
            prefix: state.tenant.redis_key_prefix.clone(),
        })
        .await;

    let algorithm = if let Some(algo) = cached_algorithm {
        algo
    } else {
        refresh_routing_cache_v1(state, key.clone(), algorithm_id, profile_id).await?
    };

    Ok(algorithm)
}

pub fn perform_straight_through_routing(
    algorithm: &routing_types::StraightThroughAlgorithm,
    creds_identifier: Option<&str>,
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

pub fn perform_routing_for_single_straight_through_algorithm(
    algorithm: &routing_types::StraightThroughAlgorithm,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    Ok(match algorithm {
        routing_types::StraightThroughAlgorithm::Single(connector) => vec![(**connector).clone()],

        routing_types::StraightThroughAlgorithm::Priority(_)
        | routing_types::StraightThroughAlgorithm::VolumeSplit(_) => {
            Err(errors::RoutingError::DslIncorrectSelectionAlgorithm)
                .attach_printable("Unsupported algorithm received as a result of static routing")?
        }
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
    state: &SessionState,
    key: String,
    algorithm_id: &common_utils::id_type::RoutingId,
    profile_id: &common_utils::id_type::ProfileId,
) -> RoutingResult<Arc<CachedAlgorithm>> {
    let algorithm = {
        let algorithm = state
            .store
            .find_routing_algorithm_by_profile_id_algorithm_id(profile_id, algorithm_id)
            .await
            .change_context(errors::RoutingError::DslMissingInDb)?;
        let algorithm: routing_types::RoutingAlgorithm = algorithm
            .algorithm_data
            .parse_value("RoutingAlgorithm")
            .change_context(errors::RoutingError::DslParsingError)?;
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

    ROUTING_CACHE
        .push(
            CacheKey {
                key,
                prefix: state.tenant.redis_key_prefix.clone(),
            },
            arc_cached_algorithm.clone(),
        )
        .await;

    Ok(arc_cached_algorithm)
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub fn perform_dynamic_routing_volume_split(
    splits: Vec<api_models::routing::RoutingVolumeSplit>,
    rng_seed: Option<&str>,
) -> RoutingResult<api_models::routing::RoutingVolumeSplit> {
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

    let routing_choice = *splits
        .get(idx)
        .ok_or(errors::RoutingError::VolumeSplitFailed)
        .attach_printable("Volume split index lookup failed")?;

    Ok(routing_choice)
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

#[cfg(feature = "v1")]
pub async fn get_merchant_cgraph(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Arc<hyperswitch_constraint_graph::ConstraintGraph<euclid_dir::DirValue>>> {
    let merchant_id = &key_store.merchant_id;

    let key = {
        match transaction_type {
            api_enums::TransactionType::Payment => {
                format!(
                    "cgraph_{}_{}",
                    merchant_id.get_string_repr(),
                    profile_id.get_string_repr()
                )
            }
            #[cfg(feature = "payouts")]
            api_enums::TransactionType::Payout => {
                format!(
                    "cgraph_po_{}_{}",
                    merchant_id.get_string_repr(),
                    profile_id.get_string_repr()
                )
            }
        }
    };

    let cached_cgraph = CGRAPH_CACHE
        .get_val::<Arc<hyperswitch_constraint_graph::ConstraintGraph<euclid_dir::DirValue>>>(
            CacheKey {
                key: key.clone(),
                prefix: state.tenant.redis_key_prefix.clone(),
            },
        )
        .await;

    let cgraph = if let Some(graph) = cached_cgraph {
        graph
    } else {
        refresh_cgraph_cache(state, key_store, key.clone(), profile_id, transaction_type).await?
    };

    Ok(cgraph)
}

#[cfg(feature = "v1")]
pub async fn refresh_cgraph_cache(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    key: String,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Arc<hyperswitch_constraint_graph::ConstraintGraph<euclid_dir::DirValue>>> {
    let mut merchant_connector_accounts = state
        .store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &state.into(),
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

    let connector_type = match transaction_type {
        api_enums::TransactionType::Payment => common_enums::ConnectorType::PaymentProcessor,
        #[cfg(feature = "payouts")]
        api_enums::TransactionType::Payout => common_enums::ConnectorType::PayoutProcessor,
    };

    let merchant_connector_accounts =
        payments_oss::helpers::filter_mca_based_on_profile_and_connector_type(
            merchant_connector_accounts,
            profile_id,
            connector_type,
        );

    let api_mcas = merchant_connector_accounts
        .into_iter()
        .map(admin_api::MerchantConnectorResponse::foreign_try_from)
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

    CGRAPH_CACHE
        .push(
            CacheKey {
                key,
                prefix: state.tenant.redis_key_prefix.clone(),
            },
            Arc::clone(&cgraph),
        )
        .await;

    Ok(cgraph)
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn perform_cgraph_filtering(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    backend_input: dsl_inputs::BackendInput,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    todo!()
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn perform_cgraph_filtering(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    backend_input: dsl_inputs::BackendInput,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let context = euclid_graph::AnalysisContext::from_dir_values(
        backend_input
            .into_context()
            .change_context(errors::RoutingError::KgraphAnalysisError)?,
    );
    let cached_cgraph = get_merchant_cgraph(state, key_store, profile_id, transaction_type).await?;

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

pub async fn perform_eligibility_analysis(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    transaction_data: &routing::TransactionData<'_>,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    profile_id: &common_utils::id_type::ProfileId,
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
        profile_id,
        &api_enums::TransactionType::from(transaction_data),
    )
    .await
}

pub async fn perform_fallback_routing(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    transaction_data: &routing::TransactionData<'_>,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    business_profile: &domain::Profile,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    #[cfg(feature = "v1")]
    let fallback_config = routing::helpers::get_merchant_default_config(
        &*state.store,
        match transaction_data {
            routing::TransactionData::Payment(payment_data) => payment_data
                .payment_intent
                .profile_id
                .as_ref()
                .get_required_value("profile_id")
                .change_context(errors::RoutingError::ProfileIdMissing)?
                .get_string_repr(),
            #[cfg(feature = "payouts")]
            routing::TransactionData::Payout(payout_data) => {
                payout_data.payout_attempt.profile_id.get_string_repr()
            }
        },
        &api_enums::TransactionType::from(transaction_data),
    )
    .await
    .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;
    #[cfg(feature = "v2")]
    let fallback_config = admin::ProfileWrapper::new(business_profile.clone())
        .get_default_fallback_list_of_connector_under_profile()
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
        business_profile.get_id(),
        &api_enums::TransactionType::from(transaction_data),
    )
    .await
}

pub async fn perform_eligibility_analysis_with_fallback(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    transaction_data: &routing::TransactionData<'_>,
    eligible_connectors: Option<Vec<api_enums::RoutableConnectors>>,
    business_profile: &domain::Profile,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let mut final_selection = perform_eligibility_analysis(
        state,
        key_store,
        chosen,
        transaction_data,
        eligible_connectors.as_ref(),
        business_profile.get_id(),
    )
    .await?;

    let fallback_selection = perform_fallback_routing(
        state,
        key_store,
        transaction_data,
        eligible_connectors.as_ref(),
        business_profile,
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

#[cfg(feature = "v2")]
pub async fn perform_session_flow_routing(
    session_input: SessionFlowRoutingInput<'_>,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<FxHashMap<api_enums::PaymentMethodType, Vec<routing_types::SessionRoutingChoice>>>
{
    todo!()
}

#[cfg(feature = "v1")]
pub async fn perform_session_flow_routing(
    session_input: SessionFlowRoutingInput<'_>,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<FxHashMap<api_enums::PaymentMethodType, Vec<routing_types::SessionRoutingChoice>>>
{
    let mut pm_type_map: FxHashMap<api_enums::PaymentMethodType, FxHashMap<String, api::GetToken>> =
        FxHashMap::default();

    #[cfg(feature = "v1")]
    let profile_id = session_input
        .payment_intent
        .profile_id
        .clone()
        .get_required_value("profile_id")
        .change_context(errors::RoutingError::ProfileIdMissing)?;

    #[cfg(feature = "v2")]
    let profile_id = session_input.payment_intent.profile_id.clone();

    let business_profile = session_input
        .state
        .store
        .find_business_profile_by_profile_id(
            &session_input.state.into(),
            session_input.key_store,
            &profile_id,
        )
        .await
        .change_context(errors::RoutingError::ProfileNotFound)?;
    #[cfg(feature = "v2")]
    let routing_algorithm =
        MerchantAccountRoutingAlgorithm::V1(business_profile.routing_algorithm_id.clone());

    #[cfg(feature = "v1")]
    let routing_algorithm: MerchantAccountRoutingAlgorithm = {
        business_profile
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

    #[cfg(feature = "v1")]
    let payment_input = dsl_inputs::PaymentInput {
        amount: session_input.payment_attempt.get_total_amount(),
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

    #[cfg(feature = "v2")]
    let payment_input = todo!();

    let metadata = session_input
        .payment_intent
        .metadata
        .clone()
        .map(|val| val.parse_value("routing_parameters"))
        .transpose()
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or(None);

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

    let mut result: FxHashMap<
        api_enums::PaymentMethodType,
        Vec<routing_types::SessionRoutingChoice>,
    > = FxHashMap::default();

    for (pm_type, allowed_connectors) in pm_type_map {
        let euclid_pmt: euclid_enums::PaymentMethodType = pm_type;
        let euclid_pm: euclid_enums::PaymentMethod = euclid_pmt.into();

        backend_input.payment_method.payment_method = Some(euclid_pm);
        backend_input.payment_method.payment_method_type = Some(euclid_pmt);

        let session_pm_input = SessionRoutingPmTypeInput {
            state: session_input.state,
            key_store: session_input.key_store,
            attempt_id: session_input.payment_attempt.get_id(),
            routing_algorithm: &routing_algorithm,
            backend_input: backend_input.clone(),
            allowed_connectors,
            profile_id: &profile_id,
        };

        let routable_connector_choice_option = perform_session_routing_for_pm_type(
            &session_pm_input,
            transaction_type,
            &business_profile,
        )
        .await?;

        if let Some(routable_connector_choice) = routable_connector_choice_option {
            let mut session_routing_choice: Vec<routing_types::SessionRoutingChoice> = Vec::new();

            for selection in routable_connector_choice {
                let connector_name = selection.connector.to_string();
                if let Some(get_token) = session_pm_input.allowed_connectors.get(&connector_name) {
                    let connector_data = api::ConnectorData::get_connector_by_name(
                        &session_pm_input.state.clone().conf.connectors,
                        &connector_name,
                        get_token.clone(),
                        selection.merchant_connector_id,
                    )
                    .change_context(errors::RoutingError::InvalidConnectorName(connector_name))?;

                    session_routing_choice.push(routing_types::SessionRoutingChoice {
                        connector: connector_data,
                        payment_method_type: pm_type,
                    });
                }
            }
            if !session_routing_choice.is_empty() {
                result.insert(pm_type, session_routing_choice);
            }
        }
    }

    Ok(result)
}

#[cfg(feature = "v1")]
async fn perform_session_routing_for_pm_type(
    session_pm_input: &SessionRoutingPmTypeInput<'_>,
    transaction_type: &api_enums::TransactionType,
    _business_profile: &domain::Profile,
) -> RoutingResult<Option<Vec<api_models::routing::RoutableConnectorChoice>>> {
    let merchant_id = &session_pm_input.key_store.merchant_id;

    let algorithm_id = match session_pm_input.routing_algorithm {
        MerchantAccountRoutingAlgorithm::V1(algorithm_ref) => &algorithm_ref.algorithm_id,
    };

    let chosen_connectors = if let Some(ref algorithm_id) = algorithm_id {
        let cached_algorithm = ensure_algorithm_cached_v1(
            &session_pm_input.state.clone(),
            merchant_id,
            algorithm_id,
            session_pm_input.profile_id,
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
        routing::helpers::get_merchant_default_config(
            &*session_pm_input.state.clone().store,
            session_pm_input.profile_id.get_string_repr(),
            transaction_type,
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed)?
    };

    let mut final_selection = perform_cgraph_filtering(
        &session_pm_input.state.clone(),
        session_pm_input.key_store,
        chosen_connectors,
        session_pm_input.backend_input.clone(),
        None,
        session_pm_input.profile_id,
        transaction_type,
    )
    .await?;

    if final_selection.is_empty() {
        let fallback = routing::helpers::get_merchant_default_config(
            &*session_pm_input.state.clone().store,
            session_pm_input.profile_id.get_string_repr(),
            transaction_type,
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

        final_selection = perform_cgraph_filtering(
            &session_pm_input.state.clone(),
            session_pm_input.key_store,
            fallback,
            session_pm_input.backend_input.clone(),
            None,
            session_pm_input.profile_id,
            transaction_type,
        )
        .await?;
    }

    if final_selection.is_empty() {
        Ok(None)
    } else {
        Ok(Some(final_selection))
    }
}

// async fn perform_session_routing_for_pm_type(
//     session_pm_input: &SessionRoutingPmTypeInput<'_>,
//     transaction_type: &api_enums::TransactionType,
//     business_profile: &domain::Profile,
// ) -> RoutingResult<Option<Vec<api_models::routing::RoutableConnectorChoice>>> {
//     let merchant_id = &session_pm_input.key_store.merchant_id;

//     let MerchantAccountRoutingAlgorithm::V1(algorithm_id) = session_pm_input.routing_algorithm;

//     let profile_wrapper = admin::ProfileWrapper::new(business_profile.clone());
//     let chosen_connectors = if let Some(ref algorithm_id) = algorithm_id {
//         let cached_algorithm = ensure_algorithm_cached_v1(
//             &session_pm_input.state.clone(),
//             merchant_id,
//             algorithm_id,
//             session_pm_input.profile_id,
//             transaction_type,
//         )
//         .await?;

//         match cached_algorithm.as_ref() {
//             CachedAlgorithm::Single(conn) => vec![(**conn).clone()],
//             CachedAlgorithm::Priority(plist) => plist.clone(),
//             CachedAlgorithm::VolumeSplit(splits) => {
//                 perform_volume_split(splits.to_vec(), Some(session_pm_input.attempt_id))
//                     .change_context(errors::RoutingError::ConnectorSelectionFailed)?
//             }
//             CachedAlgorithm::Advanced(interpreter) => execute_dsl_and_get_connector_v1(
//                 session_pm_input.backend_input.clone(),
//                 interpreter,
//             )?,
//         }
//     } else {
//         profile_wrapper
//             .get_default_fallback_list_of_connector_under_profile()
//             .change_context(errors::RoutingError::FallbackConfigFetchFailed)?
//     };

//     let mut final_selection = perform_cgraph_filtering(
//         &session_pm_input.state.clone(),
//         session_pm_input.key_store,
//         chosen_connectors,
//         session_pm_input.backend_input.clone(),
//         None,
//         session_pm_input.profile_id,
//         transaction_type,
//     )
//     .await?;

//     if final_selection.is_empty() {
//         let fallback = profile_wrapper
//             .get_default_fallback_list_of_connector_under_profile()
//             .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

//         final_selection = perform_cgraph_filtering(
//             &session_pm_input.state.clone(),
//             session_pm_input.key_store,
//             fallback,
//             session_pm_input.backend_input.clone(),
//             None,
//             session_pm_input.profile_id,
//             transaction_type,
//         )
//         .await?;
//     }

//     if final_selection.is_empty() {
//         Ok(None)
//     } else {
//         Ok(Some(final_selection))
//     }
// }
#[cfg(feature = "v2")]
pub fn make_dsl_input_for_surcharge(
    _payment_attempt: &oss_storage::PaymentAttempt,
    _payment_intent: &oss_storage::PaymentIntent,
    _billing_address: Option<Address>,
) -> RoutingResult<dsl_inputs::BackendInput> {
    todo!()
}

#[cfg(feature = "v1")]
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
        amount: payment_attempt.get_total_amount(),
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
        .unwrap_or(None);
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

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn perform_dynamic_routing(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile: &domain::Profile,
    dynamic_routing_config_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>> {
    let dynamic_routing_algo_ref: api_routing::DynamicRoutingAlgorithmRef = profile
        .dynamic_routing_algorithm
        .clone()
        .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
        .transpose()
        .change_context(errors::RoutingError::DeserializationError {
            from: "JSON".to_string(),
            to: "DynamicRoutingAlgorithmRef".to_string(),
        })
        .attach_printable("unable to deserialize DynamicRoutingAlgorithmRef from JSON")?
        .ok_or(errors::RoutingError::GenericNotFoundError {
            field: "dynamic_routing_algorithm".to_string(),
        })?;

    logger::debug!(
        "performing dynamic_routing for profile {}",
        profile.get_id().get_string_repr()
    );

    let connector_list = match dynamic_routing_algo_ref
        .success_based_algorithm
        .as_ref()
        .async_map(|algorithm| {
            perform_success_based_routing(
                state,
                routable_connectors.clone(),
                profile.get_id(),
                dynamic_routing_config_params_interpolator.clone(),
                algorithm.clone(),
            )
        })
        .await
        .transpose()
        .inspect_err(|e| logger::error!(dynamic_routing_error=?e))
        .ok()
        .flatten()
    {
        Some(success_based_list) => success_based_list,
        None => {
            // Only run contract based if success based returns None
            dynamic_routing_algo_ref
                .contract_based_routing
                .as_ref()
                .async_map(|algorithm| {
                    perform_contract_based_routing(
                        state,
                        routable_connectors.clone(),
                        profile.get_id(),
                        dynamic_routing_config_params_interpolator,
                        algorithm.clone(),
                    )
                })
                .await
                .transpose()
                .inspect_err(|e| logger::error!(dynamic_routing_error=?e))
                .ok()
                .flatten()
                .unwrap_or(routable_connectors)
        }
    };

    Ok(connector_list)
}

/// success based dynamic routing
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn perform_success_based_routing(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile_id: &common_utils::id_type::ProfileId,
    success_based_routing_config_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
    success_based_algo_ref: api_routing::SuccessBasedAlgorithm,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>> {
    if success_based_algo_ref.enabled_feature
        == api_routing::DynamicRoutingFeatures::DynamicConnectorSelection
    {
        logger::debug!(
            "performing success_based_routing for profile {}",
            profile_id.get_string_repr()
        );
        let client = state
            .grpc_client
            .dynamic_routing
            .success_rate_client
            .as_ref()
            .ok_or(errors::RoutingError::SuccessRateClientInitializationError)
            .attach_printable("success_rate gRPC client not found")?;

        let success_based_routing_configs = routing::helpers::fetch_dynamic_routing_configs::<
            api_routing::SuccessBasedRoutingConfig,
        >(
            state,
            profile_id,
            success_based_algo_ref
                .algorithm_id_with_timestamp
                .algorithm_id
                .ok_or(errors::RoutingError::GenericNotFoundError {
                    field: "success_based_routing_algorithm_id".to_string(),
                })
                .attach_printable("success_based_routing_algorithm_id not found in profile_id")?,
        )
        .await
        .change_context(errors::RoutingError::SuccessBasedRoutingConfigError)
        .attach_printable("unable to fetch success_rate based dynamic routing configs")?;

        let success_based_routing_config_params = success_based_routing_config_params_interpolator
            .get_string_val(
                success_based_routing_configs
                    .params
                    .as_ref()
                    .ok_or(errors::RoutingError::SuccessBasedRoutingParamsNotFoundError)?,
            );

        let success_based_connectors: CalSuccessRateResponse = client
            .calculate_success_rate(
                profile_id.get_string_repr().into(),
                success_based_routing_configs,
                success_based_routing_config_params,
                routable_connectors,
                state.get_grpc_headers(),
            )
            .await
            .change_context(errors::RoutingError::SuccessRateCalculationError)
            .attach_printable(
                "unable to calculate/fetch success rate from dynamic routing service",
            )?;

        let mut connectors = Vec::with_capacity(success_based_connectors.labels_with_score.len());
        for label_with_score in success_based_connectors.labels_with_score {
            let (connector, merchant_connector_id) = label_with_score.label
                .split_once(':')
                .ok_or(errors::RoutingError::InvalidSuccessBasedConnectorLabel(label_with_score.label.to_string()))
                .attach_printable(
                    "unable to split connector_name and mca_id from the label obtained by the dynamic routing service",
                )?;
            connectors.push(api_routing::RoutableConnectorChoice {
                choice_kind: api_routing::RoutableChoiceKind::FullStruct,
                connector: common_enums::RoutableConnectors::from_str(connector)
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "RoutableConnectors".to_string(),
                    })
                    .attach_printable("unable to convert String to RoutableConnectors")?,
                merchant_connector_id: Some(
                    common_utils::id_type::MerchantConnectorAccountId::wrap(
                        merchant_connector_id.to_string(),
                    )
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "MerchantConnectorAccountId".to_string(),
                    })
                    .attach_printable("unable to convert MerchantConnectorAccountId from string")?,
                ),
            });
        }
        logger::debug!(success_based_routing_connectors=?connectors);
        Ok(connectors)
    } else {
        Ok(routable_connectors)
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn perform_contract_based_routing(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile_id: &common_utils::id_type::ProfileId,
    _dynamic_routing_config_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
    contract_based_algo_ref: api_routing::ContractRoutingAlgorithm,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>> {
    if contract_based_algo_ref.enabled_feature
        == api_routing::DynamicRoutingFeatures::DynamicConnectorSelection
    {
        logger::debug!(
            "performing contract_based_routing for profile {}",
            profile_id.get_string_repr()
        );
        let client = state
            .grpc_client
            .dynamic_routing
            .contract_based_client
            .as_ref()
            .ok_or(errors::RoutingError::ContractRoutingClientInitializationError)
            .attach_printable("contract routing gRPC client not found")?;

        let contract_based_routing_configs = routing::helpers::fetch_dynamic_routing_configs::<
            api_routing::ContractBasedRoutingConfig,
        >(
            state,
            profile_id,
            contract_based_algo_ref
                .algorithm_id_with_timestamp
                .algorithm_id
                .ok_or(errors::RoutingError::GenericNotFoundError {
                    field: "contract_based_routing_algorithm_id".to_string(),
                })
                .attach_printable("contract_based_routing_algorithm_id not found in profile_id")?,
        )
        .await
        .change_context(errors::RoutingError::ContractBasedRoutingConfigError)
        .attach_printable("unable to fetch contract based dynamic routing configs")?;

        let contract_based_connectors: CalContractScoreResponse = client
            .calculate_contract_score(
                profile_id.get_string_repr().into(),
                contract_based_routing_configs,
                "".to_string(),
                routable_connectors,
                state.get_grpc_headers(),
            )
            .await
            .change_context(errors::RoutingError::ContractScoreCalculationError)
            .attach_printable(
                "unable to calculate/fetch contract score from dynamic routing service",
            )?;

        let mut connectors = Vec::with_capacity(contract_based_connectors.labels_with_score.len());

        for label_with_score in contract_based_connectors.labels_with_score {
            let (connector, merchant_connector_id) = label_with_score.label
                .split_once(':')
                .ok_or(errors::RoutingError::InvalidContractBasedConnectorLabel(label_with_score.label.to_string()))
                .attach_printable(
                    "unable to split connector_name and mca_id from the label obtained by the dynamic routing service",
                )?;

            connectors.push(api_routing::RoutableConnectorChoice {
                choice_kind: api_routing::RoutableChoiceKind::FullStruct,
                connector: common_enums::RoutableConnectors::from_str(connector)
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "RoutableConnectors".to_string(),
                    })
                    .attach_printable("unable to convert String to RoutableConnectors")?,
                merchant_connector_id: Some(
                    common_utils::id_type::MerchantConnectorAccountId::wrap(
                        merchant_connector_id.to_string(),
                    )
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "MerchantConnectorAccountId".to_string(),
                    })
                    .attach_printable("unable to convert MerchantConnectorAccountId from string")?,
                ),
            });
        }

        logger::debug!(contract_based_routing_connectors=?connectors);
        Ok(connectors)
    } else {
        Ok(routable_connectors)
    }
}
