use hyperswitch_domain_models::merchant_key_store;
use error_stack::ResultExt;
use api_models::{enums as api_enums, routing as routing_types};
use crate::errors::RouterResult;
use crate::transformers::ForeignInto;
use std::str::FromStr;
use std::sync::Arc;
use masking::PeekInterface;
use euclid::{
    backend::{ inputs as dsl_inputs},
    dssa::graph::{self as euclid_graph, CgraphExt},
    enums as euclid_enums,
    frontend::{ast, dir as euclid_dir},
};
use crate::errors::RouterResponse;
use crate::state::RoutingState;
use std::collections::HashMap;
use storage_impl::redis::cache::CGRAPH_CACHE;
use storage_impl::redis::cache::CacheKey;
use kgraph_utils::{
    mca as mca_graph,
    transformers::{IntoContext, IntoDirValue},
    types::CountryCurrencyFilter,
};
use crate::errors;
use crate::core_logic as routing;
use hyperswitch_domain_models::payment_method_data as domain;
#[cfg(feature = "v2")]
pub fn make_dsl_input(
    payments_dsl_input: &routing::PaymentsDslInput<'_>,
) -> RouterResult<dsl_inputs::BackendInput> {
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
) -> RouterResult<dsl_inputs::BackendInput> {
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
        .parse_and_get_metadata("routing_parameters")
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
// #[cfg(feature = "v1")]
pub async fn get_merchant_cgraph(
    state: &RoutingState<'_>,
    key_store: &merchant_key_store::MerchantKeyStore,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RouterResponse<Arc<hyperswitch_constraint_graph::ConstraintGraph<euclid_dir::DirValue>>> {
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

// #[cfg(feature = "v1")]
pub async fn refresh_cgraph_cache(
    state: &RoutingState<'_>,
    key_store: &merchant_key_store::MerchantKeyStore,
    key: String,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RouterResponse<Arc<hyperswitch_constraint_graph::ConstraintGraph<euclid_dir::DirValue>>> {
    let api_mcas = state
        .mca_handler
        .filter_merchant_connectors(key_store, transaction_type, profile_id)
        .await
        .change_context(errors::RoutingError::KgraphCacheRefreshFailed)
        .attach_printable("when getting merchant connectors")?;
    let connector_configs = state
        .connector_filters
        .clone()
        .into_iter()
        .filter(|(key, _)| key != "default")
        .map(|(key, value)| {
            let key = api_enums::RoutableConnectors::from_str(&key)
                .map_err(|_| errors::RoutingError::InvalidConnectorName(key))?;

            Ok((key, value))
        })
        .collect::<Result<HashMap<_, _>, errors::RoutingError>>()?;
    let default_configs = state
        .connector_filters
        .get("default");
    let config_pm_filters = CountryCurrencyFilter {
        connector_configs,
        default_configs: default_configs.cloned(),
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
    state: &RoutingState<'_>,
    key_store: &merchant_key_store::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    backend_input: dsl_inputs::BackendInput,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RouterResponse<Vec<routing_types::RoutableConnectorChoice>> {
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
