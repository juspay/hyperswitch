use std::{str::FromStr, sync::Arc};

use api_models::{
    admin::{self, PaymentMethodsEnabled},
    enums as api_enums,
    payment_methods::RequestPaymentMethodTypes,
};
use common_enums::enums;
use euclid::frontend::dir;
use hyperswitch_constraint_graph as cgraph;
use kgraph_utils::{error::KgraphError, transformers::IntoDirValue};
use storage_impl::redis::cache::PM_FILTERS_CGRAPH_CACHE;

use crate::configs::settings;

pub fn make_pm_graph(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    payment_methods: &[serde_json::value::Value],
    connector: String,
    pm_config_mapping: &settings::ConnectorFilters,
    supported_payment_methods_for_mandate: &settings::SupportedPaymentMethodsForMandate,
    supported_payment_methods_for_update_mandate: &settings::SupportedPaymentMethodsForMandate,
) -> Result<(), KgraphError> {
    for payment_method in payment_methods.iter() {
        let pm_enabled = serde_json::from_value::<PaymentMethodsEnabled>(payment_method.clone());
        if let Ok(payment_methods_enabled) = pm_enabled {
            compile_pm_graph(
                builder,
                payment_methods_enabled.clone(),
                connector.clone(),
                pm_config_mapping,
                supported_payment_methods_for_mandate,
                supported_payment_methods_for_update_mandate,
            )?;
        };
    }
    Ok(())
}

pub async fn get_merchant_pm_filter_graph<'a>(
    key: &str,
) -> Option<Arc<hyperswitch_constraint_graph::ConstraintGraph<'a, dir::DirValue>>> {
    PM_FILTERS_CGRAPH_CACHE
        .get_val::<Arc<hyperswitch_constraint_graph::ConstraintGraph<'_, dir::DirValue>>>(key)
        .await
}

pub async fn refresh_pm_filters_cache(
    key: &str,
    graph: cgraph::ConstraintGraph<'static, dir::DirValue>,
) -> Arc<hyperswitch_constraint_graph::ConstraintGraph<'static, dir::DirValue>> {
    let pm_filter_graph = Arc::new(graph);
    PM_FILTERS_CGRAPH_CACHE
        .push(key.to_string(), pm_filter_graph.clone())
        .await;
    pm_filter_graph
}

fn compile_pm_graph(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    pm_enabled: PaymentMethodsEnabled,
    connector: String,
    config: &settings::ConnectorFilters,
    supported_payment_methods_for_mandate: &settings::SupportedPaymentMethodsForMandate,
    supported_payment_methods_for_update_mandate: &settings::SupportedPaymentMethodsForMandate,
) -> Result<(), KgraphError> {
    if let Some(payment_method_types) = pm_enabled.payment_method_types {
        for pmt in payment_method_types {
            let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> =
                Vec::new();
            let mut agg_or_nodes_for_mandate_filters: Vec<(
                cgraph::NodeId,
                cgraph::Relation,
                cgraph::Strength,
            )> = Vec::new();

            // Connector supported for Update mandate filter
            let res = construct_supported_connectors_for_update_mandate_node(
                builder,
                supported_payment_methods_for_update_mandate,
                pmt.clone(),
                &pm_enabled.payment_method,
            );
            if let Ok(Some(connector_eligible_for_update_mandates_node)) = res {
                agg_or_nodes_for_mandate_filters.push((
                    connector_eligible_for_update_mandates_node,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Strong,
                ))
            }

            // Connector supported for mandates filter
            if let Some(supported_pm_for_mandates) = supported_payment_methods_for_mandate
                .0
                .get(&pm_enabled.payment_method)
            {
                if let Some(supported_connector_for_mandates) =
                    supported_pm_for_mandates.0.get(&pmt.payment_method_type)
                {
                    let supported_connectors: Vec<api_enums::Connector> =
                        supported_connector_for_mandates
                            .connector_list
                            .clone()
                            .into_iter()
                            .collect();
                    if let Ok(Some(connector_eligible_for_mandates_node)) =
                        construct_supported_connectors_for_mandate_node(
                            builder,
                            supported_connectors,
                        )
                    {
                        agg_or_nodes_for_mandate_filters.push((
                            connector_eligible_for_mandates_node,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Strong,
                        ))
                    }
                }
            }

            // Non Prominent Mandate flows
            let payment_type_non_mandate_value_node = builder.make_value_node(
                cgraph::NodeValue::Value(dir::DirValue::PaymentType(
                    euclid::enums::PaymentType::NonMandate,
                )),
                None,
                None::<()>,
            );
            let payment_type_setup_mandate_value_node = builder.make_value_node(
                cgraph::NodeValue::Value(dir::DirValue::PaymentType(
                    euclid::enums::PaymentType::SetupMandate,
                )),
                None,
                None::<()>,
            );

            let non_major_mandate_any_node = builder
                .make_any_aggregator(
                    &[
                        (
                            payment_type_non_mandate_value_node,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Strong,
                        ),
                        (
                            payment_type_setup_mandate_value_node,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Strong,
                        ),
                    ],
                    None,
                    None::<()>,
                    None,
                )
                .map_err(KgraphError::GraphConstructionError)?;

            agg_or_nodes_for_mandate_filters.push((
                non_major_mandate_any_node,
                cgraph::Relation::Positive,
                cgraph::Strength::Strong,
            ));

            let agg_or_node = builder
                .make_any_aggregator(&agg_or_nodes_for_mandate_filters, None, None::<()>, None)
                .map_err(KgraphError::GraphConstructionError)?;

            agg_nodes.push((
                agg_or_node,
                cgraph::Relation::Positive,
                cgraph::Strength::Strong,
            ));

            // Capture Method filter
            config
                .0
                .get(connector.as_str())
                .or_else(|| config.0.get("default"))
                .map(|inner| {
                    if let Ok(Some(capture_method_filter)) =
                        construct_capture_method_node(builder, inner, &pmt.payment_method_type)
                    {
                        agg_nodes.push((
                            capture_method_filter,
                            cgraph::Relation::Negative,
                            cgraph::Strength::Strong,
                        ))
                    }
                });

            // Country filter
            if let Some(pm_object_countries) = pmt.accepted_countries {
                if let Ok(Some(country_node)) = compile_accepted_countries_for_mca(
                    builder,
                    &pmt.payment_method_type,
                    pm_object_countries,
                    config,
                    connector.clone(),
                ) {
                    agg_nodes.push((
                        country_node,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Strong,
                    ))
                }
            }

            // Currency filter
            if let Some(pm_object_currencies) = pmt.accepted_currencies {
                if let Ok(Some(currency_node)) = compile_accepted_currency_for_mca(
                    builder,
                    &pmt.payment_method_type,
                    pm_object_currencies,
                    config,
                    connector.clone(),
                ) {
                    agg_nodes.push((
                        currency_node,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Strong,
                    ))
                }
            }

            let and_node_for_all_the_filters = builder
                .make_all_aggregator(&agg_nodes, None, None::<()>, None)
                .map_err(KgraphError::GraphConstructionError)?;

            // Making our output node
            let pmt_info = "PaymentMethodType";
            let dir_node: cgraph::NodeValue<dir::DirValue> =
                (pmt.payment_method_type, pm_enabled.payment_method)
                    .into_dir_value()
                    .map(Into::into)?;
            let payment_method_type_value_node =
                builder.make_value_node(dir_node, Some(pmt_info), None::<()>);

            builder
                .make_edge(
                    and_node_for_all_the_filters,
                    payment_method_type_value_node,
                    cgraph::Strength::Strong,
                    cgraph::Relation::Positive,
                    None::<cgraph::DomainId>,
                )
                .map_err(KgraphError::GraphConstructionError)?;
        }
    }
    Ok(())
}

fn construct_capture_method_node(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    payment_method_filters: &settings::PaymentMethodFilters,
    payment_method_type: &api_enums::PaymentMethodType,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    if !payment_method_filters
        .0
        .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
            *payment_method_type,
        ))
        .and_then(|v| v.not_available_flows)
        .and_then(|v| v.capture_method)
        .map(|v| !matches!(v, api_enums::CaptureMethod::Manual))
        .unwrap_or(true)
    {
        return Ok(Some(builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::CaptureMethod(
                common_enums::CaptureMethod::Manual,
            )),
            None,
            None::<()>,
        )));
    }
    Ok(None)
}

fn construct_supported_connectors_for_update_mandate_node(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    supported_payment_methods_for_update_mandate: &settings::SupportedPaymentMethodsForMandate,
    pmt: RequestPaymentMethodTypes,
    payment_method: &enums::PaymentMethod,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    let card_value_node = builder.make_value_node(
        cgraph::NodeValue::Value(dir::DirValue::PaymentMethod(enums::PaymentMethod::Card)),
        None,
        None::<()>,
    );

    let payment_type_value_node = builder.make_value_node(
        cgraph::NodeValue::Value(dir::DirValue::PaymentType(
            euclid::enums::PaymentType::UpdateMandate,
        )),
        None,
        None::<()>,
    );

    let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();
    let mut card_dir_values = Vec::new();
    let mut non_card_dir_values = Vec::new();

    if let Some(supported_pm_for_mandates) = supported_payment_methods_for_update_mandate
        .0
        .get(payment_method)
    {
        if payment_method == &enums::PaymentMethod::Card {
            if let Some(credit_connector_list) = supported_pm_for_mandates
                .0
                .get(&api_enums::PaymentMethodType::Credit)
            {
                card_dir_values.extend(
                    credit_connector_list
                        .connector_list
                        .clone()
                        .into_iter()
                        .filter_map(|connector| {
                            api_enums::RoutableConnectors::from_str(connector.to_string().as_str())
                                .ok()
                                .map(|connector| {
                                    dir::DirValue::Connector(Box::new(
                                        api_models::routing::ast::ConnectorChoice {
                                            connector,
                                            #[cfg(not(feature = "connector_choice_mca_id"))]
                                            sub_label: None,
                                        },
                                    ))
                                })
                        }),
                );
            }

            if let Some(debit_connector_list) = supported_pm_for_mandates
                .0
                .get(&api_enums::PaymentMethodType::Debit)
            {
                card_dir_values.extend(
                    debit_connector_list
                        .connector_list
                        .clone()
                        .into_iter()
                        .filter_map(|connector| {
                            api_enums::RoutableConnectors::from_str(connector.to_string().as_str())
                                .ok()
                                .map(|connector| {
                                    dir::DirValue::Connector(Box::new(
                                        api_models::routing::ast::ConnectorChoice {
                                            connector,
                                            #[cfg(not(feature = "connector_choice_mca_id"))]
                                            sub_label: None,
                                        },
                                    ))
                                })
                        }),
                );
            }
            let card_in_node = builder
                .make_in_aggregator(card_dir_values, None, None::<()>)
                .map_err(KgraphError::GraphConstructionError)?;

            let card_and_node = builder
                .make_all_aggregator(
                    &[
                        (
                            card_value_node,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Strong,
                        ),
                        (
                            payment_type_value_node,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Strong,
                        ),
                        (
                            card_in_node,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Strong,
                        ),
                    ],
                    None,
                    None::<()>,
                    None,
                )
                .map_err(KgraphError::GraphConstructionError)?;

            agg_nodes.push((
                card_and_node,
                cgraph::Relation::Positive,
                cgraph::Strength::Strong,
            ));
        } else if let Some(connector_list) =
            supported_pm_for_mandates.0.get(&pmt.payment_method_type)
        {
            non_card_dir_values.extend(
                connector_list
                    .connector_list
                    .clone()
                    .into_iter()
                    .filter_map(|connector| {
                        api_enums::RoutableConnectors::from_str(connector.to_string().as_str())
                            .ok()
                            .map(|connector| {
                                dir::DirValue::Connector(Box::new(
                                    api_models::routing::ast::ConnectorChoice {
                                        connector,
                                        #[cfg(not(feature = "connector_choice_mca_id"))]
                                        sub_label: None,
                                    },
                                ))
                            })
                    }),
            );
            let non_card_mandate_in_node = builder
                .make_in_aggregator(non_card_dir_values, None, None::<()>)
                .map_err(KgraphError::GraphConstructionError)?;

            let non_card_and_node = builder
                .make_all_aggregator(
                    &[
                        (
                            card_value_node,
                            cgraph::Relation::Negative,
                            cgraph::Strength::Strong,
                        ),
                        (
                            payment_type_value_node,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Strong,
                        ),
                        (
                            non_card_mandate_in_node,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Strong,
                        ),
                    ],
                    None,
                    None::<()>,
                    None,
                )
                .map_err(KgraphError::GraphConstructionError)?;

            agg_nodes.push((
                non_card_and_node,
                cgraph::Relation::Positive,
                cgraph::Strength::Strong,
            ));
        }
    }

    Ok(Some(
        builder
            .make_any_aggregator(
                &agg_nodes,
                Some("any node for card and non card pm"),
                None::<()>,
                None,
            )
            .map_err(KgraphError::GraphConstructionError)?,
    ))
}

fn construct_supported_connectors_for_mandate_node(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    eligible_connectors: Vec<api_enums::Connector>,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    let payment_type_value_node = builder.make_value_node(
        cgraph::NodeValue::Value(dir::DirValue::PaymentType(
            euclid::enums::PaymentType::NewMandate,
        )),
        None,
        None::<()>,
    );
    let connectors_from_config: Vec<dir::DirValue> = eligible_connectors
        .into_iter()
        .filter_map(|connector| {
            match api_enums::RoutableConnectors::from_str(connector.to_string().as_str()) {
                Ok(connector) => Some(dir::DirValue::Connector(Box::new(
                    api_models::routing::ast::ConnectorChoice {
                        connector,
                        #[cfg(not(feature = "connector_choice_mca_id"))]
                        sub_label: None,
                    },
                ))),
                Err(_) => None,
            }
        })
        .collect();

    if connectors_from_config.is_empty() {
        Ok(None)
    } else {
        let connector_in_aggregator = builder
            .make_in_aggregator(connectors_from_config, None, None::<()>)
            .map_err(KgraphError::GraphConstructionError)?;
        Ok(Some(
            builder
                .make_all_aggregator(
                    &[
                        (
                            payment_type_value_node,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Strong,
                        ),
                        (
                            connector_in_aggregator,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Strong,
                        ),
                    ],
                    None,
                    None::<()>,
                    None,
                )
                .map_err(KgraphError::GraphConstructionError)?,
        ))
    }
}

// fn construct_card_network_nodes(
//     builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
//     mca_card_networks: Vec<api_enums::CardNetwork>,
// ) -> Result<Option<cgraph::NodeId>, KgraphError> {
//     Ok(Some(
//         builder
//             .make_in_aggregator(
//                 mca_card_networks
//                     .into_iter()
//                     .map(dir::DirValue::CardNetwork)
//                     .collect(),
//                 None,
//                 None::<()>,
//             )
//             .map_err(KgraphError::GraphConstructionError)?,
//     ))
// }

fn compile_accepted_countries_for_mca(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    payment_method_type: &enums::PaymentMethodType,
    pm_obj_countries: admin::AcceptedCountries,
    config: &settings::ConnectorFilters,
    connector: String,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    match pm_obj_countries {
        admin::AcceptedCountries::EnableOnly(countries) => {
            // Country from the MCA
            let pm_object_country_value_node = builder
                .make_in_aggregator(
                    countries
                        .into_iter()
                        .map(|country| {
                            dir::DirValue::BillingCountry(common_enums::Country::from_alpha2(
                                country,
                            ))
                        })
                        .collect(),
                    None,
                    None::<()>,
                )
                .map_err(KgraphError::GraphConstructionError)?;
            if let Some(config) = config
                .0
                .get(connector.as_str())
                .or_else(|| config.0.get("default"))
            {
                if let Some(value) =
                    config
                        .0
                        .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
                            *payment_method_type,
                        ))
                {
                    // country from config
                    if let Some(config_countries) = value.country.as_ref() {
                        let config_countries: Vec<common_enums::Country> =
                            Vec::from_iter(config_countries)
                                .into_iter()
                                .map(|country| common_enums::Country::from_alpha2(*country))
                                .collect();
                        let dir_countries: Vec<dir::DirValue> = config_countries
                            .into_iter()
                            .map(dir::DirValue::BillingCountry)
                            .collect();

                        let config_country_agg_node = builder
                            .make_in_aggregator(dir_countries, None, None::<()>)
                            .map_err(KgraphError::GraphConstructionError)?;

                        let node = builder
                            .make_all_aggregator(
                                &[
                                    (
                                        pm_object_country_value_node,
                                        cgraph::Relation::Positive,
                                        cgraph::Strength::Strong,
                                    ),
                                    (
                                        config_country_agg_node,
                                        cgraph::Relation::Positive,
                                        cgraph::Strength::Strong,
                                    ),
                                ],
                                None,
                                None::<()>,
                                None,
                            )
                            .map_err(KgraphError::GraphConstructionError)?;
                        return Ok(Some(node));
                    }
                }
            }
        }
        admin::AcceptedCountries::DisableOnly(countries) => {
            if let Some(config) = config
                .0
                .get(connector.as_str())
                .or_else(|| config.0.get("default"))
            {
                // Country from the MCA
                let pm_object_country_value_node = builder
                    .make_in_aggregator(
                        countries
                            .into_iter()
                            .map(|country| {
                                dir::DirValue::BillingCountry(common_enums::Country::from_alpha2(
                                    country,
                                ))
                            })
                            .collect(),
                        None,
                        None::<()>,
                    )
                    .map_err(KgraphError::GraphConstructionError)?;

                if let Some(value) =
                    config
                        .0
                        .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
                            *payment_method_type,
                        ))
                {
                    // country from config
                    if let Some(config_countries) = value.country.as_ref() {
                        let config_countries: Vec<common_enums::Country> =
                            Vec::from_iter(config_countries)
                                .into_iter()
                                .map(|country| common_enums::Country::from_alpha2(*country))
                                .collect();
                        let dir_countries: Vec<dir::DirValue> = config_countries
                            .into_iter()
                            .map(dir::DirValue::BillingCountry)
                            .collect();

                        let config_country_agg_node = builder
                            .make_in_aggregator(dir_countries, None, None::<()>)
                            .map_err(KgraphError::GraphConstructionError)?;

                        let node = builder
                            .make_all_aggregator(
                                &[
                                    (
                                        pm_object_country_value_node,
                                        cgraph::Relation::Negative,
                                        cgraph::Strength::Strong,
                                    ),
                                    (
                                        config_country_agg_node,
                                        cgraph::Relation::Positive,
                                        cgraph::Strength::Strong,
                                    ),
                                ],
                                None,
                                None::<()>,
                                None,
                            )
                            .map_err(KgraphError::GraphConstructionError)?;
                        return Ok(Some(node));
                    }
                }
            }
        }
        admin::AcceptedCountries::AllAccepted => return Ok(None),
    }
    Ok(None)
}

fn compile_accepted_currency_for_mca(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    payment_method_type: &enums::PaymentMethodType,
    pm_obj_currency: admin::AcceptedCurrencies,
    config: &settings::ConnectorFilters,
    connector: String,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    match pm_obj_currency {
        admin::AcceptedCurrencies::EnableOnly(currency) => {
            // Currency from the MCA
            let pm_object_currency_value_node = builder
                .make_in_aggregator(
                    currency
                        .into_iter()
                        .map(dir::DirValue::PaymentCurrency)
                        .collect(),
                    None,
                    None::<()>,
                )
                .map_err(KgraphError::GraphConstructionError)?;

            if let Some(config) = config
                .0
                .get(connector.as_str())
                .or_else(|| config.0.get("default"))
            {
                if let Some(value) =
                    config
                        .0
                        .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
                            *payment_method_type,
                        ))
                {
                    // Currency from config
                    if let Some(config_currencies) = value.currency.as_ref() {
                        let config_currency: Vec<common_enums::Currency> =
                            Vec::from_iter(config_currencies)
                                .into_iter()
                                .cloned()
                                .collect();

                        let dir_currencies: Vec<dir::DirValue> = config_currency
                            .into_iter()
                            .map(dir::DirValue::PaymentCurrency)
                            .collect();

                        let config_currency_agg_node = builder
                            .make_in_aggregator(dir_currencies, None, None::<()>)
                            .map_err(KgraphError::GraphConstructionError)?;

                        let node = builder
                            .make_all_aggregator(
                                &[
                                    (
                                        pm_object_currency_value_node,
                                        cgraph::Relation::Positive,
                                        cgraph::Strength::Strong,
                                    ),
                                    (
                                        config_currency_agg_node,
                                        cgraph::Relation::Positive,
                                        cgraph::Strength::Strong,
                                    ),
                                ],
                                None,
                                None::<()>,
                                None,
                            )
                            .map_err(KgraphError::GraphConstructionError)?;
                        return Ok(Some(node));
                    }
                }
            }
        }
        admin::AcceptedCurrencies::DisableOnly(currency) => {
            // Currency from the MCA
            let pm_object_currency_value_node = builder
                .make_in_aggregator(
                    currency
                        .into_iter()
                        .map(dir::DirValue::PaymentCurrency)
                        .collect(),
                    None,
                    None::<()>,
                )
                .map_err(KgraphError::GraphConstructionError)?;

            if let Some(config) = config
                .0
                .get(connector.as_str())
                .or_else(|| config.0.get("default"))
            {
                if let Some(value) =
                    config
                        .0
                        .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
                            *payment_method_type,
                        ))
                {
                    // Currency from config
                    if let Some(config_currencies) = value.currency.as_ref() {
                        let config_currency: Vec<common_enums::Currency> =
                            Vec::from_iter(config_currencies)
                                .into_iter()
                                .cloned()
                                .collect();

                        let dir_currencies: Vec<dir::DirValue> = config_currency
                            .into_iter()
                            .map(dir::DirValue::PaymentCurrency)
                            .collect();

                        let config_currency_agg_node = builder
                            .make_in_aggregator(dir_currencies, None, None::<()>)
                            .map_err(KgraphError::GraphConstructionError)?;

                        let node = builder
                            .make_all_aggregator(
                                &[
                                    (
                                        pm_object_currency_value_node,
                                        cgraph::Relation::Negative,
                                        cgraph::Strength::Strong,
                                    ),
                                    (
                                        config_currency_agg_node,
                                        cgraph::Relation::Positive,
                                        cgraph::Strength::Strong,
                                    ),
                                ],
                                None,
                                None::<()>,
                                None,
                            )
                            .map_err(KgraphError::GraphConstructionError)?;
                        return Ok(Some(node));
                    }
                }
            }
        }
        admin::AcceptedCurrencies::AllAccepted => return Ok(None),
    }
    Ok(None)
}
