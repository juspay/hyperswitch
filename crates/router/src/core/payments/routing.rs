mod transformers;
pub mod utils;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use std::collections::hash_map;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use std::hash::{Hash, Hasher};
use std::{collections::HashMap, str::FromStr, sync::Arc};

#[cfg(feature = "v1")]
use api_models::open_router::{self as or_types, DecidedGateway, OpenRouterDecideGatewayRequest};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::routing as api_routing;
use api_models::{
    admin as admin_api,
    enums::{self as api_enums, CountryAlpha2},
    routing::ConnectorSelection,
};
use common_types::payments as common_payments_types;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
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
    contract_routing_client::ContractBasedDynamicRouting,
    elimination_based_client::EliminationBasedRouting,
    success_rate_client::SuccessBasedDynamicRouting, DynamicRoutingError,
};
use hyperswitch_domain_models::address::Address;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use hyperswitch_interfaces::events::routing_api_logs::{ApiMethod, RoutingEngine};
use kgraph_utils::{
    mca as mca_graph,
    transformers::{IntoContext, IntoDirValue},
    types::CountryCurrencyFilter,
};
use masking::{PeekInterface, Secret};
use rand::distributions::{self, Distribution};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use rand::SeedableRng;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use router_env::{instrument, tracing};
use rustc_hash::FxHashMap;
use storage_impl::redis::cache::{CacheKey, CGRAPH_CACHE, ROUTING_CACHE};

#[cfg(feature = "v2")]
use crate::core::admin;
#[cfg(feature = "payouts")]
use crate::core::payouts;
#[cfg(feature = "v1")]
use crate::core::routing::transformers::OpenRouterDecideGatewayRequestExt;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use crate::routes::app::SessionStateInfo;
use crate::{
    core::{
        errors, errors as oss_errors,
        payments::{
            routing::utils::DecisionEngineApiHandler, OperationSessionGetters,
            OperationSessionSetters,
        },
        routing,
    },
    logger, services,
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

#[cfg(feature = "v1")]
pub struct SessionFlowRoutingInput<'a> {
    pub state: &'a SessionState,
    pub country: Option<CountryAlpha2>,
    pub key_store: &'a domain::MerchantKeyStore,
    pub merchant_account: &'a domain::MerchantAccount,
    pub payment_attempt: &'a oss_storage::PaymentAttempt,
    pub payment_intent: &'a oss_storage::PaymentIntent,
    pub chosen: api::SessionConnectorDatas,
}

#[cfg(feature = "v2")]
pub struct SessionFlowRoutingInput<'a> {
    pub country: Option<CountryAlpha2>,
    pub payment_intent: &'a oss_storage::PaymentIntent,
    pub chosen: api::SessionConnectorDatas,
}

#[allow(dead_code)]
#[cfg(feature = "v1")]
pub struct SessionRoutingPmTypeInput<'a> {
    state: &'a SessionState,
    key_store: &'a domain::MerchantKeyStore,
    attempt_id: &'a str,
    routing_algorithm: &'a MerchantAccountRoutingAlgorithm,
    backend_input: dsl_inputs::BackendInput,
    allowed_connectors: FxHashMap<String, api::GetToken>,
    profile_id: &'a common_utils::id_type::ProfileId,
}

#[cfg(feature = "v2")]
pub struct SessionRoutingPmTypeInput<'a> {
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
        extended_card_bin: None,
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
            .and_then(|ba| ba.address.as_ref())
            .and_then(|addr| addr.country)
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
            .map(api_enums::PaymentMethodType::foreign_from)
            .or_else(|| {
                payout_data.payment_method.as_ref().and_then(|pm| {
                    #[cfg(feature = "v1")]
                    {
                        pm.payment_method_type
                    }
                    #[cfg(feature = "v2")]
                    {
                        pm.payment_method_subtype
                    }
                })
            }),
        card_network: None,
    };
    Ok(dsl_inputs::BackendInput {
        mandate,
        metadata,
        payment,
        payment_method,
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
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
                        common_payments_types::AcceptanceType::Online => {
                            euclid_enums::MandateAcceptanceType::Online
                        }
                        common_payments_types::AcceptanceType::Offline => {
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
        extended_card_bin: payments_dsl_input
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => {
                    Some(card.card_number.peek().chars().take(8).collect::<String>())
                }
                _ => None,
            }),
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
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
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
                        common_payments_types::AcceptanceType::Online => {
                            euclid_enums::MandateAcceptanceType::Online
                        }
                        common_payments_types::AcceptanceType::Offline => {
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
        extended_card_bin: payments_dsl_input
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => {
                    Some(card.card_number.peek().chars().take(8).collect())
                }
                _ => None,
            }),
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
        .parse_and_get_metadata("routing_parameters")
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or(None);

    Ok(dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: mandate_data,
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
    })
}

pub async fn perform_static_routing_v1(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    algorithm_id: Option<&common_utils::id_type::RoutingId>,
    business_profile: &domain::Profile,
    transaction_data: &routing::TransactionData<'_>,
) -> RoutingResult<(
    Vec<routing_types::RoutableConnectorChoice>,
    Option<common_enums::RoutingApproach>,
)> {
    logger::debug!("euclid_routing: performing routing for connector selection");
    let get_merchant_fallback_config = || async {
        #[cfg(feature = "v1")]
        return routing::helpers::get_merchant_default_config(
            &*state.clone().store,
            business_profile.get_id().get_string_repr(),
            &api_enums::TransactionType::from(transaction_data),
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed);
        #[cfg(feature = "v2")]
        return admin::ProfileWrapper::new(business_profile.clone())
            .get_default_fallback_list_of_connector_under_profile()
            .change_context(errors::RoutingError::FallbackConfigFetchFailed);
    };

    let fallback_config = get_merchant_fallback_config().await?;

    let algorithm_id = if let Some(id) = algorithm_id {
        id
    } else {
        logger::debug!("euclid_routing: active algorithm isn't present, default falling back");
        return Ok((fallback_config, None));
    };

    let cached_algorithm = match ensure_algorithm_cached_v1(
        state,
        merchant_id,
        algorithm_id,
        business_profile.get_id(),
        &api_enums::TransactionType::from(transaction_data),
    )
    .await
    {
        Ok(algo) => algo,
        Err(err) => {
            logger::error!(
                error=?err,
                "euclid_routing: ensure_algorithm_cached failed, falling back to merchant default connectors"
            );

            return Ok((fallback_config, None));
        }
    };

    let backend_input = match transaction_data {
        routing::TransactionData::Payment(payment_data) => make_dsl_input(payment_data)?,
        #[cfg(feature = "payouts")]
        routing::TransactionData::Payout(payout_data) => make_dsl_input_for_payouts(payout_data)?,
    };

    let payment_id = match transaction_data {
        routing::TransactionData::Payment(payment_data) => payment_data
            .payment_attempt
            .payment_id
            .clone()
            .get_string_repr()
            .to_string(),
        #[cfg(feature = "payouts")]
        routing::TransactionData::Payout(payout_data) => payout_data
            .payout_attempt
            .payout_id
            .get_string_repr()
            .to_string(),
    };

    // Decision of de-routing is stored
    let de_evaluated_connector = if !state.conf.open_router.static_routing_enabled {
        logger::debug!("decision_engine_euclid: decision_engine routing not enabled");
        Vec::default()
    } else {
        utils::decision_engine_routing(
            state,
            backend_input.clone(),
            business_profile,
            payment_id,
            fallback_config,
        )
        .await
        .map_err(|e|
            // errors are ignored as this is just for diff checking as of now (optional flow).
            logger::error!(decision_engine_euclid_evaluate_error=?e, "decision_engine_euclid: error in evaluation of rule")
        )
        .unwrap_or_default()
    };

    let (routable_connectors, routing_approach) = match cached_algorithm.as_ref() {
        CachedAlgorithm::Single(conn) => (
            vec![(**conn).clone()],
            Some(common_enums::RoutingApproach::StraightThroughRouting),
        ),
        CachedAlgorithm::Priority(plist) => (plist.clone(), None),
        CachedAlgorithm::VolumeSplit(splits) => (
            perform_volume_split(splits.to_vec())
                .change_context(errors::RoutingError::ConnectorSelectionFailed)?,
            Some(common_enums::RoutingApproach::VolumeBasedRouting),
        ),
        CachedAlgorithm::Advanced(interpreter) => (
            execute_dsl_and_get_connector_v1(backend_input, interpreter)?,
            Some(common_enums::RoutingApproach::RuleBasedRouting),
        ),
    };

    // Results are logged for diff(between legacy and decision_engine's euclid) and have parameters as:
    // is_equal: verifies all output are matching in order,
    // is_equal_length: matches length of both outputs (useful for verifying volume based routing
    // results)
    // de_response: response from the decision_engine's euclid
    // hs_response: response from legacy_euclid
    utils::compare_and_log_result(
        de_evaluated_connector.clone(),
        routable_connectors.clone(),
        "evaluate_routing".to_string(),
    );

    Ok((
        utils::select_routing_result(
            state,
            business_profile,
            routable_connectors,
            de_evaluated_connector,
        )
        .await,
        routing_approach,
    ))
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
            common_enums::TransactionType::ThreeDsAuthentication => {
                Err(errors::RoutingError::InvalidTransactionType)?
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
            perform_volume_split(splits.to_vec())
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
    let routing_output: routing_types::StaticRoutingAlgorithm = interpreter
        .execute(backend_input)
        .map(|out| out.connector_selection.foreign_into())
        .change_context(errors::RoutingError::DslExecutionError)?;

    Ok(match routing_output {
        routing_types::StaticRoutingAlgorithm::Priority(plist) => plist,

        routing_types::StaticRoutingAlgorithm::VolumeSplit(splits) => perform_volume_split(splits)
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
        let algorithm: routing_types::StaticRoutingAlgorithm = algorithm
            .algorithm_data
            .parse_value("RoutingAlgorithm")
            .change_context(errors::RoutingError::DslParsingError)?;
        algorithm
    };

    let cached_algorithm = match algorithm {
        routing_types::StaticRoutingAlgorithm::Single(conn) => CachedAlgorithm::Single(conn),
        routing_types::StaticRoutingAlgorithm::Priority(plist) => CachedAlgorithm::Priority(plist),
        routing_types::StaticRoutingAlgorithm::VolumeSplit(splits) => {
            CachedAlgorithm::VolumeSplit(splits)
        }
        routing_types::StaticRoutingAlgorithm::Advanced(program) => {
            let interpreter = backend::VirInterpreterBackend::with_program(program)
                .change_context(errors::RoutingError::DslBackendInitError)
                .attach_printable("Error initializing DSL interpreter backend")?;

            CachedAlgorithm::Advanced(interpreter)
        }
        api_models::routing::StaticRoutingAlgorithm::ThreeDsDecisionRule(_program) => {
            Err(errors::RoutingError::InvalidRoutingAlgorithmStructure)
                .attach_printable("Unsupported algorithm received")?
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
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let weights: Vec<u8> = splits.iter().map(|sp| sp.split).collect();
    let weighted_index = distributions::WeightedIndex::new(weights)
        .change_context(errors::RoutingError::VolumeSplitFailed)
        .attach_printable("Error creating weighted distribution for volume split")?;

    let mut rng = rand::thread_rng();
    let idx = weighted_index.sample(&mut rng);

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

// #[cfg(feature = "v1")]
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
            api_enums::TransactionType::ThreeDsAuthentication => {
                Err(errors::RoutingError::InvalidTransactionType)?
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

// #[cfg(feature = "v1")]
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
        api_enums::TransactionType::ThreeDsAuthentication => {
            Err(errors::RoutingError::InvalidTransactionType)?
        }
    };

    let connector_type = match transaction_type {
        api_enums::TransactionType::Payment => common_enums::ConnectorType::PaymentProcessor,
        #[cfg(feature = "payouts")]
        api_enums::TransactionType::Payout => common_enums::ConnectorType::PayoutProcessor,
        api_enums::TransactionType::ThreeDsAuthentication => {
            Err(errors::RoutingError::InvalidTransactionType)?
        }
    };

    let merchant_connector_accounts = merchant_connector_accounts
        .filter_based_on_profile_and_connector_type(profile_id, connector_type);

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

#[allow(clippy::too_many_arguments)]
pub async fn perform_cgraph_filtering(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    backend_input: dsl_inputs::BackendInput,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
    active_mca_ids: &std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let context = euclid_graph::AnalysisContext::from_dir_values(
        backend_input
            .into_context()
            .change_context(errors::RoutingError::KgraphAnalysisError)?,
    );

    let cached_cgraph = get_merchant_cgraph(state, key_store, profile_id, transaction_type).await?;

    let mut final_selection = Vec::new();

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
            eligible_connectors.is_none_or(|list| list.contains(&routable_connector));

        let mca_active = choice
            .merchant_connector_id
            .as_ref()
            .map(|id| active_mca_ids.contains(id))
            .unwrap_or(false);

        if cgraph_eligible && filter_eligible && mca_active {
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

    let active_mca_ids = get_active_mca_ids(state, key_store).await?;
    perform_cgraph_filtering(
        state,
        key_store,
        chosen,
        backend_input,
        eligible_connectors,
        profile_id,
        &api_enums::TransactionType::from(transaction_data),
        &active_mca_ids,
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
    let active_mca_ids = get_active_mca_ids(state, key_store).await?;
    perform_cgraph_filtering(
        state,
        key_store,
        fallback_config,
        backend_input,
        eligible_connectors,
        business_profile.get_id(),
        &api_enums::TransactionType::from(transaction_data),
        &active_mca_ids,
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
    logger::debug!("euclid_routing: performing eligibility");
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
    logger::debug!(final_selected_connectors_for_routing=?final_selected_connectors, "euclid_routing: List of final selected connectors for routing");

    Ok(final_selection)
}

#[cfg(feature = "v2")]
pub async fn perform_session_flow_routing<'a>(
    state: &'a SessionState,
    key_store: &'a domain::MerchantKeyStore,
    session_input: SessionFlowRoutingInput<'_>,
    business_profile: &domain::Profile,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<FxHashMap<api_enums::PaymentMethodType, Vec<routing_types::SessionRoutingChoice>>>
{
    let mut pm_type_map: FxHashMap<api_enums::PaymentMethodType, FxHashMap<String, api::GetToken>> =
        FxHashMap::default();

    let profile_id = business_profile.get_id().clone();

    let routing_algorithm =
        MerchantAccountRoutingAlgorithm::V1(business_profile.routing_algorithm_id.clone());

    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: None,
        payment_method_type: None,
        card_network: None,
    };

    let payment_input = dsl_inputs::PaymentInput {
        amount: session_input
            .payment_intent
            .amount_details
            .calculate_net_amount(),
        currency: session_input.payment_intent.amount_details.currency,
        authentication_type: session_input.payment_intent.authentication_type,
        card_bin: None,
        extended_card_bin: None,
        capture_method: Option::<euclid_enums::CaptureMethod>::foreign_from(
            session_input.payment_intent.capture_method,
        ),
        // business_country not available in payment_intent anymore
        business_country: None,
        billing_country: session_input
            .country
            .map(storage_enums::Country::from_alpha2),
        // business_label not available in payment_intent anymore
        business_label: None,
        setup_future_usage: Some(session_input.payment_intent.setup_future_usage),
    };

    let metadata = session_input
        .payment_intent
        .parse_and_get_metadata("routing_parameters")
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
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
    };

    for connector_data in session_input.chosen.iter() {
        pm_type_map
            .entry(connector_data.payment_method_sub_type)
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
    let active_mca_ids = get_active_mca_ids(state, key_store).await?;

    for (pm_type, allowed_connectors) in pm_type_map {
        let euclid_pmt: euclid_enums::PaymentMethodType = pm_type;
        let euclid_pm: euclid_enums::PaymentMethod = euclid_pmt.into();

        backend_input.payment_method.payment_method = Some(euclid_pm);
        backend_input.payment_method.payment_method_type = Some(euclid_pmt);

        let session_pm_input = SessionRoutingPmTypeInput {
            routing_algorithm: &routing_algorithm,
            backend_input: backend_input.clone(),
            allowed_connectors,
            profile_id: &profile_id,
        };

        let routable_connector_choice_option = perform_session_routing_for_pm_type(
            state,
            key_store,
            &session_pm_input,
            transaction_type,
            business_profile,
            &active_mca_ids,
        )
        .await?;

        if let Some(routable_connector_choice) = routable_connector_choice_option {
            let mut session_routing_choice: Vec<routing_types::SessionRoutingChoice> = Vec::new();

            for selection in routable_connector_choice {
                let connector_name = selection.connector.to_string();
                if let Some(get_token) = session_pm_input.allowed_connectors.get(&connector_name) {
                    let connector_data = api::ConnectorData::get_connector_by_name(
                        &state.clone().conf.connectors,
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
pub async fn perform_session_flow_routing(
    session_input: SessionFlowRoutingInput<'_>,
    business_profile: &domain::Profile,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<(
    FxHashMap<api_enums::PaymentMethodType, Vec<routing_types::SessionRoutingChoice>>,
    Option<common_enums::RoutingApproach>,
)> {
    let mut pm_type_map: FxHashMap<api_enums::PaymentMethodType, FxHashMap<String, api::GetToken>> =
        FxHashMap::default();

    let profile_id = session_input
        .payment_intent
        .profile_id
        .clone()
        .get_required_value("profile_id")
        .change_context(errors::RoutingError::ProfileIdMissing)?;

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
        extended_card_bin: None,
        capture_method: session_input
            .payment_attempt
            .capture_method
            .and_then(Option::<euclid_enums::CaptureMethod>::foreign_from),
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
        .parse_and_get_metadata("routing_parameters")
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
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
    };

    for connector_data in session_input.chosen.iter() {
        pm_type_map
            .entry(connector_data.payment_method_sub_type)
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
    let mut final_routing_approach = None;
    let active_mca_ids = get_active_mca_ids(session_input.state, session_input.key_store).await?;

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

        let (routable_connector_choice_option, routing_approach) =
            perform_session_routing_for_pm_type(
                &session_pm_input,
                transaction_type,
                business_profile,
                &active_mca_ids,
            )
            .await?;

        final_routing_approach = routing_approach;

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

    Ok((result, final_routing_approach))
}

#[cfg(feature = "v1")]
async fn perform_session_routing_for_pm_type(
    session_pm_input: &SessionRoutingPmTypeInput<'_>,
    transaction_type: &api_enums::TransactionType,
    _business_profile: &domain::Profile,
    active_mca_ids: &std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId>,
) -> RoutingResult<(
    Option<Vec<api_models::routing::RoutableConnectorChoice>>,
    Option<common_enums::RoutingApproach>,
)> {
    let merchant_id = &session_pm_input.key_store.merchant_id;

    let algorithm_id = match session_pm_input.routing_algorithm {
        MerchantAccountRoutingAlgorithm::V1(algorithm_ref) => &algorithm_ref.algorithm_id,
    };

    let (chosen_connectors, routing_approach) = if let Some(ref algorithm_id) = algorithm_id {
        let cached_algorithm = ensure_algorithm_cached_v1(
            &session_pm_input.state.clone(),
            merchant_id,
            algorithm_id,
            session_pm_input.profile_id,
            transaction_type,
        )
        .await?;

        match cached_algorithm.as_ref() {
            CachedAlgorithm::Single(conn) => (
                vec![(**conn).clone()],
                Some(common_enums::RoutingApproach::StraightThroughRouting),
            ),
            CachedAlgorithm::Priority(plist) => (plist.clone(), None),
            CachedAlgorithm::VolumeSplit(splits) => (
                perform_volume_split(splits.to_vec())
                    .change_context(errors::RoutingError::ConnectorSelectionFailed)?,
                Some(common_enums::RoutingApproach::VolumeBasedRouting),
            ),
            CachedAlgorithm::Advanced(interpreter) => (
                execute_dsl_and_get_connector_v1(
                    session_pm_input.backend_input.clone(),
                    interpreter,
                )?,
                Some(common_enums::RoutingApproach::RuleBasedRouting),
            ),
        }
    } else {
        (
            routing::helpers::get_merchant_default_config(
                &*session_pm_input.state.clone().store,
                session_pm_input.profile_id.get_string_repr(),
                transaction_type,
            )
            .await
            .change_context(errors::RoutingError::FallbackConfigFetchFailed)?,
            None,
        )
    };

    let mut final_selection = perform_cgraph_filtering(
        &session_pm_input.state.clone(),
        session_pm_input.key_store,
        chosen_connectors,
        session_pm_input.backend_input.clone(),
        None,
        session_pm_input.profile_id,
        transaction_type,
        active_mca_ids,
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
            active_mca_ids,
        )
        .await?;
    }

    if final_selection.is_empty() {
        Ok((None, routing_approach))
    } else {
        Ok((Some(final_selection), routing_approach))
    }
}

#[cfg(feature = "v2")]
async fn get_chosen_connectors<'a>(
    state: &'a SessionState,
    key_store: &'a domain::MerchantKeyStore,
    session_pm_input: &SessionRoutingPmTypeInput<'_>,
    transaction_type: &api_enums::TransactionType,
    profile_wrapper: &admin::ProfileWrapper,
) -> RoutingResult<Vec<api_models::routing::RoutableConnectorChoice>> {
    let merchant_id = &key_store.merchant_id;

    let MerchantAccountRoutingAlgorithm::V1(algorithm_id) = session_pm_input.routing_algorithm;

    let chosen_connectors = if let Some(ref algorithm_id) = algorithm_id {
        let cached_algorithm = ensure_algorithm_cached_v1(
            state,
            merchant_id,
            algorithm_id,
            session_pm_input.profile_id,
            transaction_type,
        )
        .await?;

        match cached_algorithm.as_ref() {
            CachedAlgorithm::Single(conn) => vec![(**conn).clone()],
            CachedAlgorithm::Priority(plist) => plist.clone(),
            CachedAlgorithm::VolumeSplit(splits) => perform_volume_split(splits.to_vec())
                .change_context(errors::RoutingError::ConnectorSelectionFailed)?,
            CachedAlgorithm::Advanced(interpreter) => execute_dsl_and_get_connector_v1(
                session_pm_input.backend_input.clone(),
                interpreter,
            )?,
        }
    } else {
        profile_wrapper
            .get_default_fallback_list_of_connector_under_profile()
            .change_context(errors::RoutingError::FallbackConfigFetchFailed)?
    };
    Ok(chosen_connectors)
}

#[cfg(feature = "v2")]
async fn perform_session_routing_for_pm_type<'a>(
    state: &'a SessionState,
    key_store: &'a domain::MerchantKeyStore,
    session_pm_input: &SessionRoutingPmTypeInput<'_>,
    transaction_type: &api_enums::TransactionType,
    business_profile: &domain::Profile,
    active_mca_ids: &std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId>,
) -> RoutingResult<Option<Vec<api_models::routing::RoutableConnectorChoice>>> {
    let profile_wrapper = admin::ProfileWrapper::new(business_profile.clone());
    let chosen_connectors = get_chosen_connectors(
        state,
        key_store,
        session_pm_input,
        transaction_type,
        &profile_wrapper,
    )
    .await?;

    let mut final_selection = perform_cgraph_filtering(
        state,
        key_store,
        chosen_connectors,
        session_pm_input.backend_input.clone(),
        None,
        session_pm_input.profile_id,
        transaction_type,
        active_mca_ids,
    )
    .await?;

    if final_selection.is_empty() {
        let fallback = profile_wrapper
            .get_default_fallback_list_of_connector_under_profile()
            .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

        final_selection = perform_cgraph_filtering(
            state,
            key_store,
            fallback,
            session_pm_input.backend_input.clone(),
            None,
            session_pm_input.profile_id,
            transaction_type,
            active_mca_ids,
        )
        .await?;
    }

    if final_selection.is_empty() {
        Ok(None)
    } else {
        Ok(Some(final_selection))
    }
}
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
        extended_card_bin: None,
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
        .parse_and_get_metadata("routing_parameters")
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
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
    };
    Ok(backend_input)
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn perform_dynamic_routing_with_open_router<F, D>(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile: &domain::Profile,
    payment_data: oss_storage::PaymentAttempt,
    old_payment_data: &mut D,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
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
        "performing dynamic_routing with open_router for profile {}",
        profile.get_id().get_string_repr()
    );

    let is_success_rate_routing_enabled =
        dynamic_routing_algo_ref.is_success_rate_routing_enabled();
    let is_elimination_enabled = dynamic_routing_algo_ref.is_elimination_enabled();

    // Since success_based and elimination routing is being done in 1 api call, we call decide_gateway when either of it enabled
    let connectors = if is_success_rate_routing_enabled || is_elimination_enabled {
        let connectors = perform_decide_gateway_call_with_open_router(
            state,
            routable_connectors.clone(),
            profile.get_id(),
            &payment_data,
            is_elimination_enabled,
            old_payment_data,
        )
        .await?;

        if is_elimination_enabled {
            // This will initiate the elimination process for the connector.
            // Penalize the elimination score of the connector before making a payment.
            // Once the payment is made, we will update the score based on the payment status
            if let Some(connector) = connectors.first() {
                logger::debug!(
                "penalizing the elimination score of the gateway with id {} in open_router for profile {}",
                connector, profile.get_id().get_string_repr()
            );
                update_gateway_score_with_open_router(
                    state,
                    connector.clone(),
                    profile.get_id(),
                    &payment_data.merchant_id,
                    &payment_data.payment_id,
                    common_enums::AttemptStatus::AuthenticationPending,
                )
                .await?
            }
        }
        connectors
    } else {
        routable_connectors
    };

    Ok(connectors)
}

#[cfg(feature = "v1")]
pub async fn perform_open_routing_for_debit_routing<F, D>(
    state: &SessionState,
    co_badged_card_request: or_types::CoBadgedCardRequest,
    card_isin: Option<Secret<String>>,
    old_payment_data: &mut D,
) -> RoutingResult<or_types::DebitRoutingOutput>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let payment_attempt = old_payment_data.get_payment_attempt().clone();

    logger::debug!(
        "performing debit routing with open_router for profile {}",
        payment_attempt.profile_id.get_string_repr()
    );

    let metadata = Some(
        serde_json::to_string(&co_badged_card_request)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode Vaulting data to string")
            .change_context(errors::RoutingError::MetadataParsingError)?,
    );

    let open_router_req_body = OpenRouterDecideGatewayRequest::construct_debit_request(
        &payment_attempt,
        metadata,
        card_isin,
        Some(or_types::RankingAlgorithm::NtwBasedRouting),
    );

    let routing_events_wrapper = utils::RoutingEventsWrapper::new(
        state.tenant.tenant_id.clone(),
        state.request_id.clone(),
        payment_attempt.payment_id.get_string_repr().to_string(),
        payment_attempt.profile_id.to_owned(),
        payment_attempt.merchant_id.to_owned(),
        "DecisionEngine: Debit Routing".to_string(),
        Some(open_router_req_body.clone()),
        true,
        true,
    );

    let response: RoutingResult<utils::RoutingEventsResponse<DecidedGateway>> =
        utils::EuclidApiClient::send_decision_engine_request(
            state,
            services::Method::Post,
            "decide-gateway",
            Some(open_router_req_body),
            None,
            Some(routing_events_wrapper),
        )
        .await;

    let output = match response {
        Ok(events_response) => {
            let response =
                events_response
                    .response
                    .ok_or(errors::RoutingError::OpenRouterError(
                        "Response from decision engine API is empty".to_string(),
                    ))?;

            let debit_routing_output = response
                .debit_routing_output
                .get_required_value("debit_routing_output")
                .change_context(errors::RoutingError::OpenRouterError(
                    "Failed to parse the response from open_router".into(),
                ))
                .attach_printable("debit_routing_output is missing in the open routing response")?;

            old_payment_data.set_routing_approach_in_attempt(Some(
                common_enums::RoutingApproach::from_decision_engine_approach(
                    &response.routing_approach,
                ),
            ));

            Ok(debit_routing_output)
        }
        Err(error_response) => {
            logger::error!("open_router_error_response: {:?}", error_response);
            Err(errors::RoutingError::OpenRouterError(
                "Failed to perform debit routing in open router".into(),
            ))
        }
    }?;

    Ok(output)
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn perform_dynamic_routing_with_intelligent_router<F, D>(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile: &domain::Profile,
    dynamic_routing_config_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
    payment_data: &mut D,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
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

    let payment_attempt = payment_data.get_payment_attempt().clone();

    let mut connector_list = match dynamic_routing_algo_ref
        .success_based_algorithm
        .as_ref()
        .async_map(|algorithm| {
            perform_success_based_routing(
                state,
                routable_connectors.clone(),
                profile.get_id(),
                &payment_attempt.merchant_id,
                &payment_attempt.payment_id,
                dynamic_routing_config_params_interpolator.clone(),
                algorithm.clone(),
                payment_data,
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
                        &payment_attempt.merchant_id,
                        &payment_attempt.payment_id,
                        dynamic_routing_config_params_interpolator.clone(),
                        algorithm.clone(),
                        payment_data,
                    )
                })
                .await
                .transpose()
                .inspect_err(|e| logger::error!(dynamic_routing_error=?e))
                .ok()
                .flatten()
                .unwrap_or(routable_connectors.clone())
        }
    };

    connector_list = dynamic_routing_algo_ref
        .elimination_routing_algorithm
        .as_ref()
        .async_map(|algorithm| {
            perform_elimination_routing(
                state,
                connector_list.clone(),
                profile.get_id(),
                &payment_attempt.merchant_id,
                &payment_attempt.payment_id,
                dynamic_routing_config_params_interpolator.clone(),
                algorithm.clone(),
            )
        })
        .await
        .transpose()
        .inspect_err(|e| logger::error!(dynamic_routing_error=?e))
        .ok()
        .flatten()
        .unwrap_or(connector_list);

    Ok(connector_list)
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn perform_decide_gateway_call_with_open_router<F, D>(
    state: &SessionState,
    mut routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile_id: &common_utils::id_type::ProfileId,
    payment_attempt: &oss_storage::PaymentAttempt,
    is_elimination_enabled: bool,
    old_payment_data: &mut D,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    logger::debug!(
        "performing decide_gateway call with open_router for profile {}",
        profile_id.get_string_repr()
    );

    let open_router_req_body = OpenRouterDecideGatewayRequest::construct_sr_request(
        payment_attempt,
        routable_connectors.clone(),
        Some(or_types::RankingAlgorithm::SrBasedRouting),
        is_elimination_enabled,
    );

    let routing_events_wrapper = utils::RoutingEventsWrapper::new(
        state.tenant.tenant_id.clone(),
        state.request_id.clone(),
        payment_attempt.payment_id.get_string_repr().to_string(),
        payment_attempt.profile_id.to_owned(),
        payment_attempt.merchant_id.to_owned(),
        "DecisionEngine: SuccessRate decide_gateway".to_string(),
        Some(open_router_req_body.clone()),
        true,
        false,
    );

    let response: RoutingResult<utils::RoutingEventsResponse<DecidedGateway>> =
        utils::SRApiClient::send_decision_engine_request(
            state,
            services::Method::Post,
            "decide-gateway",
            Some(open_router_req_body),
            None,
            Some(routing_events_wrapper),
        )
        .await;

    let sr_sorted_connectors = match response {
        Ok(resp) => {
            let decided_gateway: DecidedGateway =
                resp.response.ok_or(errors::RoutingError::OpenRouterError(
                    "Empty response received from open_router".into(),
                ))?;

            let mut routing_event = resp.event.ok_or(errors::RoutingError::RoutingEventsError {
                message: "Decision-Engine: RoutingEvent not found in RoutingEventsResponse"
                    .to_string(),
                status_code: 500,
            })?;

            routing_event.set_response_body(&decided_gateway);
            routing_event.set_routing_approach(
                utils::RoutingApproach::from_decision_engine_approach(
                    &decided_gateway.routing_approach,
                )
                .to_string(),
            );

            old_payment_data.set_routing_approach_in_attempt(Some(
                common_enums::RoutingApproach::from_decision_engine_approach(
                    &decided_gateway.routing_approach,
                ),
            ));

            if let Some(gateway_priority_map) = decided_gateway.gateway_priority_map {
                logger::debug!(gateway_priority_map=?gateway_priority_map, routing_approach=decided_gateway.routing_approach, "open_router decide_gateway call response");
                routable_connectors.sort_by(|connector_choice_a, connector_choice_b| {
                    let connector_choice_a_score = gateway_priority_map
                        .get(&connector_choice_a.to_string())
                        .copied()
                        .unwrap_or(0.0);
                    let connector_choice_b_score = gateway_priority_map
                        .get(&connector_choice_b.to_string())
                        .copied()
                        .unwrap_or(0.0);
                    connector_choice_b_score
                        .partial_cmp(&connector_choice_a_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            routing_event.set_routable_connectors(routable_connectors.clone());
            state.event_handler().log_event(&routing_event);

            Ok(routable_connectors)
        }
        Err(err) => {
            logger::error!("open_router_error_response: {:?}", err);

            Err(errors::RoutingError::OpenRouterError(
                "Failed to perform decide_gateway call in open_router".into(),
            ))
        }
    }?;

    Ok(sr_sorted_connectors)
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn update_gateway_score_with_open_router(
    state: &SessionState,
    payment_connector: api_routing::RoutableConnectorChoice,
    profile_id: &common_utils::id_type::ProfileId,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_id: &common_utils::id_type::PaymentId,
    payment_status: common_enums::AttemptStatus,
) -> RoutingResult<()> {
    let open_router_req_body = or_types::UpdateScorePayload {
        merchant_id: profile_id.clone(),
        gateway: payment_connector.to_string(),
        status: payment_status.foreign_into(),
        payment_id: payment_id.clone(),
    };

    let routing_events_wrapper = utils::RoutingEventsWrapper::new(
        state.tenant.tenant_id.clone(),
        state.request_id.clone(),
        payment_id.get_string_repr().to_string(),
        profile_id.to_owned(),
        merchant_id.to_owned(),
        "DecisionEngine: SuccessRate update_gateway_score".to_string(),
        Some(open_router_req_body.clone()),
        true,
        false,
    );

    let response: RoutingResult<utils::RoutingEventsResponse<or_types::UpdateScoreResponse>> =
        utils::SRApiClient::send_decision_engine_request(
            state,
            services::Method::Post,
            "update-gateway-score",
            Some(open_router_req_body),
            None,
            Some(routing_events_wrapper),
        )
        .await;

    match response {
        Ok(resp) => {
            let update_score_resp = resp.response.ok_or(errors::RoutingError::OpenRouterError(
                "Failed to parse the response from open_router".into(),
            ))?;

            let mut routing_event = resp.event.ok_or(errors::RoutingError::RoutingEventsError {
                message: "Decision-Engine: RoutingEvent not found in RoutingEventsResponse"
                    .to_string(),
                status_code: 500,
            })?;

            logger::debug!(
                "open_router update_gateway_score response for gateway with id {}: {:?}",
                payment_connector,
                update_score_resp.message
            );

            routing_event.set_response_body(&update_score_resp);
            routing_event.set_payment_connector(payment_connector.clone()); // check this in review
            state.event_handler().log_event(&routing_event);

            Ok(())
        }
        Err(err) => {
            logger::error!("open_router_update_gateway_score_error: {:?}", err);

            Err(errors::RoutingError::OpenRouterError(
                "Failed to update gateway score in open_router".into(),
            ))
        }
    }?;

    Ok(())
}

/// success based dynamic routing
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn perform_success_based_routing<F, D>(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile_id: &common_utils::id_type::ProfileId,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_id: &common_utils::id_type::PaymentId,
    success_based_routing_config_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
    success_based_algo_ref: api_routing::SuccessBasedAlgorithm,
    payment_data: &mut D,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    if success_based_algo_ref.enabled_feature
        == api_routing::DynamicRoutingFeatures::DynamicConnectorSelection
    {
        logger::debug!(
            "performing success_based_routing for profile {}",
            profile_id.get_string_repr()
        );
        let client = &state
            .grpc_client
            .dynamic_routing
            .as_ref()
            .ok_or(errors::RoutingError::SuccessRateClientInitializationError)
            .attach_printable("dynamic routing gRPC client not found")?
            .success_rate_client;

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

        let event_request = utils::CalSuccessRateEventRequest {
            id: profile_id.get_string_repr().to_string(),
            params: success_based_routing_config_params.clone(),
            labels: routable_connectors
                .iter()
                .map(|conn_choice| conn_choice.to_string())
                .collect::<Vec<_>>(),
            config: success_based_routing_configs
                .config
                .as_ref()
                .map(utils::CalSuccessRateConfigEventRequest::from),
        };

        let routing_events_wrapper = utils::RoutingEventsWrapper::new(
            state.tenant.tenant_id.clone(),
            state.request_id.clone(),
            payment_id.get_string_repr().to_string(),
            profile_id.to_owned(),
            merchant_id.to_owned(),
            "IntelligentRouter: CalculateSuccessRate".to_string(),
            Some(event_request.clone()),
            true,
            false,
        );

        let closure = || async {
            let success_based_connectors_result = client
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
                );

            match success_based_connectors_result {
                Ok(success_response) => {
                    let updated_resp = utils::CalSuccessRateEventResponse::try_from(
                        &success_response,
                    )
                    .change_context(errors::RoutingError::RoutingEventsError { message: "unable to convert SuccessBasedConnectors to CalSuccessRateEventResponse".to_string(), status_code: 500 })
                    .attach_printable(
                        "unable to convert SuccessBasedConnectors to CalSuccessRateEventResponse",
                    )?;

                    Ok(Some(updated_resp))
                }
                Err(e) => {
                    logger::error!(
                        "unable to calculate/fetch success rate from dynamic routing service: {:?}",
                        e.current_context()
                    );

                    Err(error_stack::report!(
                        errors::RoutingError::SuccessRateCalculationError
                    ))
                }
            }
        };

        let events_response = routing_events_wrapper
            .construct_event_builder(
                "SuccessRateCalculator.FetchSuccessRate".to_string(),
                RoutingEngine::IntelligentRouter,
                ApiMethod::Grpc,
            )?
            .trigger_event(state, closure)
            .await?;

        let success_based_connectors: utils::CalSuccessRateEventResponse = events_response
            .response
            .ok_or(errors::RoutingError::SuccessRateCalculationError)?;

        // Need to log error case
        let mut routing_event =
            events_response
                .event
                .ok_or(errors::RoutingError::RoutingEventsError {
                    message:
                        "SR-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse"
                            .to_string(),
                    status_code: 500,
                })?;

        routing_event.set_routing_approach(success_based_connectors.routing_approach.to_string());
        payment_data.set_routing_approach_in_attempt(Some(common_enums::RoutingApproach::from(
            success_based_connectors.routing_approach,
        )));

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

        routing_event.set_status_code(200);
        routing_event.set_routable_connectors(connectors.clone());
        state.event_handler().log_event(&routing_event);
        Ok(connectors)
    } else {
        Ok(routable_connectors)
    }
}

/// elimination dynamic routing
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn perform_elimination_routing(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile_id: &common_utils::id_type::ProfileId,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_id: &common_utils::id_type::PaymentId,
    elimination_routing_configs_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
    elimination_algo_ref: api_routing::EliminationRoutingAlgorithm,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>> {
    if elimination_algo_ref.enabled_feature
        == api_routing::DynamicRoutingFeatures::DynamicConnectorSelection
    {
        logger::debug!(
            "performing elimination_routing for profile {}",
            profile_id.get_string_repr()
        );
        let client = &state
            .grpc_client
            .dynamic_routing
            .as_ref()
            .ok_or(errors::RoutingError::EliminationClientInitializationError)
            .attach_printable("dynamic routing gRPC client not found")?
            .elimination_based_client;

        let elimination_routing_config = routing::helpers::fetch_dynamic_routing_configs::<
            api_routing::EliminationRoutingConfig,
        >(
            state,
            profile_id,
            elimination_algo_ref
                .algorithm_id_with_timestamp
                .algorithm_id
                .ok_or(errors::RoutingError::GenericNotFoundError {
                    field: "elimination_routing_algorithm_id".to_string(),
                })
                .attach_printable(
                    "elimination_routing_algorithm_id not found in business_profile",
                )?,
        )
        .await
        .change_context(errors::RoutingError::EliminationRoutingConfigError)
        .attach_printable("unable to fetch elimination dynamic routing configs")?;

        let elimination_routing_config_params = elimination_routing_configs_params_interpolator
            .get_string_val(
                elimination_routing_config
                    .params
                    .as_ref()
                    .ok_or(errors::RoutingError::EliminationBasedRoutingParamsNotFoundError)?,
            );

        let event_request = utils::EliminationRoutingEventRequest {
            id: profile_id.get_string_repr().to_string(),
            params: elimination_routing_config_params.clone(),
            labels: routable_connectors
                .iter()
                .map(|conn_choice| conn_choice.to_string())
                .collect::<Vec<_>>(),
            config: elimination_routing_config
                .elimination_analyser_config
                .as_ref()
                .map(utils::EliminationRoutingEventBucketConfig::from),
        };

        let routing_events_wrapper = utils::RoutingEventsWrapper::new(
            state.tenant.tenant_id.clone(),
            state.request_id.clone(),
            payment_id.get_string_repr().to_string(),
            profile_id.to_owned(),
            merchant_id.to_owned(),
            "IntelligentRouter: PerformEliminationRouting".to_string(),
            Some(event_request.clone()),
            true,
            false,
        );

        let closure = || async {
            let elimination_based_connectors_result = client
                .perform_elimination_routing(
                    profile_id.get_string_repr().to_string(),
                    elimination_routing_config_params,
                    routable_connectors.clone(),
                    elimination_routing_config.elimination_analyser_config,
                    state.get_grpc_headers(),
                )
                .await
                .change_context(errors::RoutingError::EliminationRoutingCalculationError)
                .attach_printable(
                    "unable to analyze/fetch elimination routing from dynamic routing service",
                );

            match elimination_based_connectors_result {
                Ok(elimination_response) => Ok(Some(utils::EliminationEventResponse::from(
                    &elimination_response,
                ))),
                Err(e) => {
                    logger::error!(
                        "unable to analyze/fetch elimination routing from dynamic routing service: {:?}",
                        e.current_context()
                    );

                    Err(error_stack::report!(
                        errors::RoutingError::EliminationRoutingCalculationError
                    ))
                }
            }
        };

        let events_response = routing_events_wrapper
            .construct_event_builder(
                "EliminationAnalyser.GetEliminationStatus".to_string(),
                RoutingEngine::IntelligentRouter,
                ApiMethod::Grpc,
            )?
            .trigger_event(state, closure)
            .await?;

        let elimination_based_connectors: utils::EliminationEventResponse = events_response
            .response
            .ok_or(errors::RoutingError::EliminationRoutingCalculationError)?;

        let mut routing_event = events_response
            .event
            .ok_or(errors::RoutingError::RoutingEventsError {
            message:
                "Elimination-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse"
                    .to_string(),
            status_code: 500,
        })?;

        routing_event.set_routing_approach(utils::RoutingApproach::Elimination.to_string());

        let mut connectors =
            Vec::with_capacity(elimination_based_connectors.labels_with_status.len());
        let mut eliminated_connectors =
            Vec::with_capacity(elimination_based_connectors.labels_with_status.len());
        let mut non_eliminated_connectors =
            Vec::with_capacity(elimination_based_connectors.labels_with_status.len());
        for labels_with_status in elimination_based_connectors.labels_with_status {
            let (connector, merchant_connector_id) = labels_with_status.label
                .split_once(':')
                .ok_or(errors::RoutingError::InvalidEliminationBasedConnectorLabel(labels_with_status.label.to_string()))
                .attach_printable(
                    "unable to split connector_name and mca_id from the label obtained by the elimination based dynamic routing service",
                )?;

            let routable_connector = api_routing::RoutableConnectorChoice {
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
            };

            if labels_with_status
                .elimination_information
                .is_some_and(|elimination_info| {
                    elimination_info
                        .entity
                        .is_some_and(|entity_info| entity_info.is_eliminated)
                })
            {
                eliminated_connectors.push(routable_connector);
            } else {
                non_eliminated_connectors.push(routable_connector);
            }
            connectors.extend(non_eliminated_connectors.clone());
            connectors.extend(eliminated_connectors.clone());
        }
        logger::debug!(dynamic_eliminated_connectors=?eliminated_connectors);
        logger::debug!(dynamic_elimination_based_routing_connectors=?connectors);

        routing_event.set_status_code(200);
        routing_event.set_routable_connectors(connectors.clone());
        state.event_handler().log_event(&routing_event);
        Ok(connectors)
    } else {
        Ok(routable_connectors)
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[allow(clippy::too_many_arguments)]
pub async fn perform_contract_based_routing<F, D>(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile_id: &common_utils::id_type::ProfileId,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_id: &common_utils::id_type::PaymentId,
    _dynamic_routing_config_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
    contract_based_algo_ref: api_routing::ContractRoutingAlgorithm,
    payment_data: &mut D,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    if contract_based_algo_ref.enabled_feature
        == api_routing::DynamicRoutingFeatures::DynamicConnectorSelection
    {
        logger::debug!(
            "performing contract_based_routing for profile {}",
            profile_id.get_string_repr()
        );
        let client = &state
            .grpc_client
            .dynamic_routing
            .as_ref()
            .ok_or(errors::RoutingError::ContractRoutingClientInitializationError)
            .attach_printable("dynamic routing gRPC client not found")?
            .contract_based_client;

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

        let label_info = contract_based_routing_configs
            .label_info
            .clone()
            .ok_or(errors::RoutingError::ContractBasedRoutingConfigError)
            .attach_printable("Label information not found in contract routing configs")?;

        let contract_based_connectors = routable_connectors
            .clone()
            .into_iter()
            .filter(|conn| {
                label_info
                    .iter()
                    .any(|info| Some(info.mca_id.clone()) == conn.merchant_connector_id.clone())
            })
            .collect::<Vec<_>>();

        let mut other_connectors = routable_connectors
            .into_iter()
            .filter(|conn| {
                label_info
                    .iter()
                    .all(|info| Some(info.mca_id.clone()) != conn.merchant_connector_id.clone())
            })
            .collect::<Vec<_>>();

        let event_request = utils::CalContractScoreEventRequest {
            id: profile_id.get_string_repr().to_string(),
            params: "".to_string(),
            labels: contract_based_connectors
                .iter()
                .map(|conn_choice| conn_choice.to_string())
                .collect::<Vec<_>>(),
            config: Some(contract_based_routing_configs.clone()),
        };

        let routing_events_wrapper = utils::RoutingEventsWrapper::new(
            state.tenant.tenant_id.clone(),
            state.request_id.clone(),
            payment_id.get_string_repr().to_string(),
            profile_id.to_owned(),
            merchant_id.to_owned(),
            "IntelligentRouter: PerformContractRouting".to_string(),
            Some(event_request.clone()),
            true,
            false,
        );

        let closure = || async {
            let contract_based_connectors_result = client
                .calculate_contract_score(
                    profile_id.get_string_repr().into(),
                    contract_based_routing_configs.clone(),
                    "".to_string(),
                    contract_based_connectors,
                    state.get_grpc_headers(),
                )
                .await
                .attach_printable(
                    "unable to calculate/fetch contract score from dynamic routing service",
                );

            let contract_based_connectors = match contract_based_connectors_result {
                Ok(resp) => Some(utils::CalContractScoreEventResponse::from(&resp)),
                Err(err) => match err.current_context() {
                    DynamicRoutingError::ContractNotFound => {
                        client
                            .update_contracts(
                                profile_id.get_string_repr().into(),
                                label_info,
                                "".to_string(),
                                vec![],
                                u64::default(),
                                state.get_grpc_headers(),
                            )
                            .await
                            .change_context(errors::RoutingError::ContractScoreUpdationError)
                            .attach_printable(
                                "unable to update contract based routing window in dynamic routing service",
                            )?;
                        return Err((errors::RoutingError::ContractScoreCalculationError {
                            err: err.to_string(),
                        })
                        .into());
                    }
                    _ => {
                        return Err((errors::RoutingError::ContractScoreCalculationError {
                            err: err.to_string(),
                        })
                        .into())
                    }
                },
            };

            Ok(contract_based_connectors)
        };

        let events_response = routing_events_wrapper
            .construct_event_builder(
                "ContractScoreCalculator.FetchContractScore".to_string(),
                RoutingEngine::IntelligentRouter,
                ApiMethod::Grpc,
            )?
            .trigger_event(state, closure)
            .await?;

        let contract_based_connectors: utils::CalContractScoreEventResponse = events_response
            .response
            .ok_or(errors::RoutingError::ContractScoreCalculationError {
                err: "CalContractScoreEventResponse not found".to_string(),
            })?;

        let mut routing_event = events_response
            .event
            .ok_or(errors::RoutingError::RoutingEventsError {
            message:
                "ContractRouting-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse"
                    .to_string(),
            status_code: 500,
        })?;

        payment_data.set_routing_approach_in_attempt(Some(
            common_enums::RoutingApproach::ContractBasedRouting,
        ));

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

        connectors.append(&mut other_connectors);

        logger::debug!(contract_based_routing_connectors=?connectors);

        routing_event.set_status_code(200);
        routing_event.set_routable_connectors(connectors.clone());
        routing_event.set_routing_approach(api_routing::RoutingApproach::ContractBased.to_string());
        state.event_handler().log_event(&routing_event);
        Ok(connectors)
    } else {
        Ok(routable_connectors)
    }
}

pub async fn get_active_mca_ids(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
) -> RoutingResult<std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId>> {
    let db_mcas = state
        .store
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &key_store.merchant_id,
            false,
            key_store,
        )
        .await
        .unwrap_or_else(|_| {
            hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccounts::new(
                vec![],
            )
        });

    let active_mca_ids: std::collections::HashSet<_> =
        db_mcas.iter().map(|mca| mca.get_id().clone()).collect();
    Ok(active_mca_ids)
}
