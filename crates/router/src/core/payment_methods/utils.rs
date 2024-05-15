use api_models::admin::{self, PaymentMethodsEnabled};
use cgraph::NodeValue;
pub use common_enums::enums;
use common_enums::PaymentMethodType;
use euclid::frontend::dir::{self, DirValue};
use hyperswitch_constraint_graph as cgraph;
use kgraph_utils::{error::KgraphError, transformers::IntoDirValue};

use crate::configs::settings;

pub fn make_pm_graph<'a>(
    builder: &mut cgraph::ConstraintGraphBuilder<'a, DirValue>,
    payment_methods: Vec<serde_json::value::Value>,
    connector: String,
    pm_config_mapping: &settings::ConnectorFilters,
) -> Result<(), KgraphError> {
    for payment_method in payment_methods.into_iter() {
        let pm_enabled = serde_json::from_value::<PaymentMethodsEnabled>(payment_method);
        if let Ok(payment_methods_enabled) = pm_enabled {
            compile_pm_graph(
                builder,
                payment_methods_enabled.clone(),
                connector.clone(),
                pm_config_mapping,
            )?;
        };
    }
    Ok(())
}

fn compile_pm_graph(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, DirValue>,
    pm_enabled: PaymentMethodsEnabled,
    connector: String,
    config: &settings::ConnectorFilters,
) -> Result<(), KgraphError> {
    let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();
    if let Some(payment_method_types) = pm_enabled.payment_method_types {
        for pmt in payment_method_types {
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

            let and_node_for_country_and_currency_filters = builder
                .make_all_aggregator(&agg_nodes, None, None::<()>, None)
                .map_err(KgraphError::GraphConstructionError)?;

            // Making our output node
            let pmt_info = "PaymentMethodType";
            let dir_node :NodeValue<DirValue> = (pmt.payment_method_type, pm_enabled.payment_method).into_dir_value().map(Into::into)?;
            let payment_method_type_value_node = builder.make_value_node(
                dir_node,
                Some(pmt_info),
                None::<()>,
            );

            builder
                .make_edge(
                    and_node_for_country_and_currency_filters,
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

fn compile_accepted_countries_for_mca(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, DirValue>,
    payment_method_type: &PaymentMethodType,
    pm_obj_countries: admin::AcceptedCountries,
    config: &settings::ConnectorFilters,
    connector: String,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    match pm_obj_countries {
        admin::AcceptedCountries::EnableOnly(countries) => {
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
                    println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>mca countries {:?}", countries);
                    // Country from the MCA
                    let pm_object_country_value_node = builder
                        .make_in_aggregator(
                            countries
                                .into_iter()
                                .map(|country| {
                                    dir::DirValue::BillingCountry(
                                        common_enums::Country::from_alpha2(country),
                                    )
                                })
                                .collect(),
                            None,
                            None::<()>,
                        )
                        .map_err(KgraphError::GraphConstructionError)?;

                    // country from config
                    if let Some(config_countries) = value.country.as_ref() {
                        let config_countries: Vec<common_enums::Country> =
                            Vec::from_iter(config_countries)
                                .into_iter()
                                .map(|country| common_enums::Country::from_alpha2(*country))
                                .collect();
                        println!(
                            ">>>>>>>>>>>>>>>>>>>>>>>>>>>config countries {:?}",
                            config_countries
                        );
                        let dir_countries: Vec<DirValue> = config_countries
                            .into_iter()
                            .map(|country| dir::DirValue::BillingCountry(country))
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
        admin::AcceptedCountries::DisableOnly(_) => todo!(),
        admin::AcceptedCountries::AllAccepted => todo!(),
    }
    Ok(None)
}

fn compile_accepted_currency_for_mca(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, DirValue>,
    payment_method_type: &PaymentMethodType,
    pm_obj_currency: admin::AcceptedCurrencies,
    config: &settings::ConnectorFilters,
    connector: String,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    match pm_obj_currency {
        admin::AcceptedCurrencies::EnableOnly(currency) => {
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
                    println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>mca currencies {:?}", currency);
                    // Currency from the MCA
                    let pm_object_currency_value_node = builder
                        .make_in_aggregator(
                            currency
                                .into_iter()
                                .map(|currency| dir::DirValue::PaymentCurrency(currency))
                                .collect(),
                            None,
                            None::<()>,
                        )
                        .map_err(KgraphError::GraphConstructionError)?;

                    // Currency from config
                    if let Some(config_currencies) = value.currency.as_ref() {
                        let config_currency: Vec<common_enums::Currency> =
                            Vec::from_iter(config_currencies)
                                .into_iter()
                                .cloned()
                                .collect();

                        println!(
                            ">>>>>>>>>>>>>>>>>>>>>>>>>>>config currency {:?}",
                            config_currency
                        );
                        let dir_currencies: Vec<DirValue> = config_currency
                            .into_iter()
                            .map(|currency| dir::DirValue::PaymentCurrency(currency))
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
        admin::AcceptedCurrencies::DisableOnly(_) => todo!(),
        admin::AcceptedCurrencies::AllAccepted => todo!(),
    }
    Ok(None)
}
