use std::str::FromStr;

use api_models::{
    admin as admin_api, enums as api_enums, payment_methods::RequestPaymentMethodTypes,
};
use euclid::{
    frontend::{ast, dir},
    types::{NumValue, NumValueRefinement},
};
use hyperswitch_constraint_graph as cgraph;

use crate::{
    error::KgraphError,
    transformers::IntoDirValue,
    utils::{CountryCurrencyFilter, CurrencyCountryFlowFilter, PaymentMethodFilterKey},
};

pub const DOMAIN_IDENTIFIER: &str = "payment_methods_enabled_for_merchantconnectoraccount";

fn compile_request_pm_types(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    pm_types: RequestPaymentMethodTypes,
    pm: api_enums::PaymentMethod,
) -> Result<cgraph::NodeId, KgraphError> {
    let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();

    let pmt_info = "PaymentMethodType";
    let pmt_id = builder.make_value_node(
        (pm_types.payment_method_type, pm)
            .into_dir_value()
            .map(Into::into)?,
        Some(pmt_info),
        None::<()>,
    );
    agg_nodes.push((
        pmt_id,
        cgraph::Relation::Positive,
        match pm_types.payment_method_type {
            api_enums::PaymentMethodType::Credit | api_enums::PaymentMethodType::Debit => {
                cgraph::Strength::Weak
            }

            _ => cgraph::Strength::Strong,
        },
    ));

    if let Some(card_networks) = pm_types.card_networks {
        if !card_networks.is_empty() {
            let dir_vals: Vec<dir::DirValue> = card_networks
                .into_iter()
                .map(IntoDirValue::into_dir_value)
                .collect::<Result<_, _>>()?;

            let card_network_info = "Card Networks";
            let card_network_id = builder
                .make_in_aggregator(dir_vals, Some(card_network_info), None::<()>)
                .map_err(KgraphError::GraphConstructionError)?;

            agg_nodes.push((
                card_network_id,
                cgraph::Relation::Positive,
                cgraph::Strength::Weak,
            ));
        }
    }

    let currencies_data = pm_types
        .accepted_currencies
        .and_then(|accepted_currencies| match accepted_currencies {
            admin_api::AcceptedCurrencies::EnableOnly(curr) if !curr.is_empty() => Some((
                curr.into_iter()
                    .map(IntoDirValue::into_dir_value)
                    .collect::<Result<_, _>>()
                    .ok()?,
                cgraph::Relation::Positive,
            )),

            admin_api::AcceptedCurrencies::DisableOnly(curr) if !curr.is_empty() => Some((
                curr.into_iter()
                    .map(IntoDirValue::into_dir_value)
                    .collect::<Result<_, _>>()
                    .ok()?,
                cgraph::Relation::Negative,
            )),

            _ => None,
        });

    if let Some((currencies, relation)) = currencies_data {
        let accepted_currencies_info = "Accepted Currencies";
        let accepted_currencies_id = builder
            .make_in_aggregator(currencies, Some(accepted_currencies_info), None::<()>)
            .map_err(KgraphError::GraphConstructionError)?;

        agg_nodes.push((accepted_currencies_id, relation, cgraph::Strength::Strong));
    }

    let mut amount_nodes = Vec::with_capacity(2);

    if let Some(min_amt) = pm_types.minimum_amount {
        let num_val = NumValue {
            number: min_amt.into(),
            refinement: Some(NumValueRefinement::GreaterThanEqual),
        };

        let min_amt_info = "Minimum Amount";
        let min_amt_id = builder.make_value_node(
            dir::DirValue::PaymentAmount(num_val).into(),
            Some(min_amt_info),
            None::<()>,
        );

        amount_nodes.push(min_amt_id);
    }

    if let Some(max_amt) = pm_types.maximum_amount {
        let num_val = NumValue {
            number: max_amt.into(),
            refinement: Some(NumValueRefinement::LessThanEqual),
        };

        let max_amt_info = "Maximum Amount";
        let max_amt_id = builder.make_value_node(
            dir::DirValue::PaymentAmount(num_val).into(),
            Some(max_amt_info),
            None::<()>,
        );

        amount_nodes.push(max_amt_id);
    }

    if !amount_nodes.is_empty() {
        let zero_num_val = NumValue {
            number: 0,
            refinement: None,
        };

        let zero_amt_id = builder.make_value_node(
            dir::DirValue::PaymentAmount(zero_num_val).into(),
            Some("zero_amount"),
            None::<()>,
        );

        let or_node_neighbor_id = if amount_nodes.len() == 1 {
            amount_nodes
                .first()
                .copied()
                .ok_or(KgraphError::IndexingError)?
        } else {
            let nodes = amount_nodes
                .iter()
                .copied()
                .map(|node_id| {
                    (
                        node_id,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Strong,
                    )
                })
                .collect::<Vec<_>>();

            builder
                .make_all_aggregator(
                    &nodes,
                    Some("amount_constraint_aggregator"),
                    None::<()>,
                    None,
                )
                .map_err(KgraphError::GraphConstructionError)?
        };

        let any_aggregator = builder
            .make_any_aggregator(
                &[
                    (
                        zero_amt_id,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Strong,
                    ),
                    (
                        or_node_neighbor_id,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Strong,
                    ),
                ],
                Some("zero_plus_limits_amount_aggregator"),
                None::<()>,
                None,
            )
            .map_err(KgraphError::GraphConstructionError)?;

        agg_nodes.push((
            any_aggregator,
            cgraph::Relation::Positive,
            cgraph::Strength::Strong,
        ));
    }

    let pmt_all_aggregator_info = "All Aggregator for PaymentMethodType";
    builder
        .make_all_aggregator(&agg_nodes, Some(pmt_all_aggregator_info), None::<()>, None)
        .map_err(KgraphError::GraphConstructionError)
}

fn compile_payment_method_enabled(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    enabled: admin_api::PaymentMethodsEnabled,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    let agg_id = if !enabled
        .payment_method_types
        .as_ref()
        .map(|v| v.is_empty())
        .unwrap_or(true)
    {
        let pm_info = "PaymentMethod";
        let pm_id = builder.make_value_node(
            enabled.payment_method.into_dir_value().map(Into::into)?,
            Some(pm_info),
            None::<()>,
        );

        let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();

        if let Some(pm_types) = enabled.payment_method_types {
            for pm_type in pm_types {
                let node_id = compile_request_pm_types(builder, pm_type, enabled.payment_method)?;
                agg_nodes.push((
                    node_id,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Strong,
                ));
            }
        }

        let any_aggregator_info = "Any aggregation for PaymentMethodsType";
        let pm_type_agg_id = builder
            .make_any_aggregator(&agg_nodes, Some(any_aggregator_info), None::<()>, None)
            .map_err(KgraphError::GraphConstructionError)?;

        let all_aggregator_info = "All aggregation for PaymentMethod";
        let enabled_pm_agg_id = builder
            .make_all_aggregator(
                &[
                    (pm_id, cgraph::Relation::Positive, cgraph::Strength::Strong),
                    (
                        pm_type_agg_id,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Strong,
                    ),
                ],
                Some(all_aggregator_info),
                None::<()>,
                None,
            )
            .map_err(KgraphError::GraphConstructionError)?;

        Some(enabled_pm_agg_id)
    } else {
        None
    };

    Ok(agg_id)
}
fn compile_graph_for_countries(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    config: &CurrencyCountryFlowFilter,
    payment_method_type_node: cgraph::NodeId,
) -> Result<cgraph::NodeId, KgraphError> {
    let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();
    agg_nodes.push((
        payment_method_type_node,
        cgraph::Relation::Positive,
        cgraph::Strength::Normal,
    ));
    if let Some(country) = config.country.clone() {
        let node_country = country
            .into_iter()
            .map(|country| api_enums::Country::from_alpha2(country))
            .map(IntoDirValue::into_dir_value)
            .collect::<Result<Vec<_>, _>>()?;
        let country_agg = builder
            .make_in_aggregator(node_country, Some("Configs for Country"), None::<()>)
            .map_err(KgraphError::GraphConstructionError)?;
        agg_nodes.push((
            country_agg,
            cgraph::Relation::Positive,
            cgraph::Strength::Weak,
        ))
    }

    if let Some(currency) = config.currency.clone() {
        let node_currency = currency
            .into_iter()
            .map(IntoDirValue::into_dir_value)
            .collect::<Result<Vec<_>, _>>()?;
        let currency_agg = builder
            .make_in_aggregator(node_currency, Some("Configs for Currency"), None::<()>)
            .map_err(KgraphError::GraphConstructionError)?;
        agg_nodes.push((
            currency_agg,
            cgraph::Relation::Positive,
            cgraph::Strength::Strong,
        ))
    }
    if let Some(capture_method) = config
        .not_available_flows
        .and_then(|naf| naf.capture_method)
    {
        let make_capture_node = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::CaptureMethod(capture_method)),
            Some("Configs for Currency"),
            None::<()>,
        );
        agg_nodes.push((
            make_capture_node,
            cgraph::Relation::Negative,
            cgraph::Strength::Normal,
        ))
    }

    Ok(builder
        .make_all_aggregator(
            &agg_nodes,
            Some("Country & Currency Configs"),
            None::<()>,
            None,
        )
        .map_err(KgraphError::GraphConstructionError)?)
}
fn compile_config_graph(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    config: &CountryCurrencyFilter,
    connector: &api_enums::RoutableConnectors,
) -> Result<cgraph::NodeId, KgraphError> {
    let mut agg_node_id: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();

    if let Some(pmt) = config
        .connector_configs
        .get(connector)
        .or_else(|| config.default_configs.as_ref())
        .map(|inner| inner.0.clone())
    {
        for key in pmt.keys().cloned() {
            match key {
                PaymentMethodFilterKey::PaymentMethodType(pm) => {
                    let pmt_id = builder.make_value_node(
                        pm.into_dir_value().map(Into::into)?,
                        Some("PaymentMethodType"),
                        None::<()>,
                    );
                    let curr = pmt
                        .get(&PaymentMethodFilterKey::PaymentMethodType(pm))
                        .map(|country| compile_graph_for_countries(builder, country, pmt_id))
                        .transpose()?;

                    if let Some(country_currency) = curr {
                        agg_node_id.push((
                            country_currency,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Normal,
                        ));
                    }
                }
                PaymentMethodFilterKey::CardNetwork(cn) => {
                    let cn_id = builder.make_value_node(
                        cn.clone().into_dir_value().map(Into::into)?,
                        Some("CardNetwork"),
                        None::<()>,
                    );
                    let curr = pmt
                        .get(&PaymentMethodFilterKey::CardNetwork(cn.clone()))
                        .map(|country| compile_graph_for_countries(builder, country, cn_id))
                        .transpose()?;

                    if let Some(_country_currency) = curr {
                        agg_node_id.push((
                            cn_id,
                            cgraph::Relation::Positive,
                            cgraph::Strength::Normal,
                        ));
                    }
                }
            }
        }
    }

    let info = "Config";
    builder
        .make_any_aggregator(&agg_node_id, Some(info), None::<()>, None)
        .map_err(KgraphError::GraphConstructionError)
}

fn compile_merchant_connector_graph(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, dir::DirValue>,
    mca: admin_api::MerchantConnectorResponse,
    config: &CountryCurrencyFilter,
) -> Result<(), KgraphError> {
    let connector = common_enums::RoutableConnectors::from_str(&mca.connector_name)
        .map_err(|_| KgraphError::InvalidConnectorName(mca.connector_name.clone()))?;

    let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();

    if let Some(pms_enabled) = mca.payment_methods_enabled.clone() {
        for pm_enabled in pms_enabled {
            let maybe_pm_enabled_id = compile_payment_method_enabled(builder, pm_enabled)?;
            if let Some(pm_enabled_id) = maybe_pm_enabled_id.clone() {
                agg_nodes.push((
                    pm_enabled_id,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Strong,
                ));
            }
        }
    }

    let aggregator_info = "Available Payment methods for connector";
    let pms_enabled_agg_id = builder
        .make_any_aggregator(&agg_nodes, Some(aggregator_info), None::<()>, None)
        .map_err(KgraphError::GraphConstructionError)?;

    let config_info = "Config for respective PaymentMethodType for the connector";

    let config_enabled_agg_id = compile_config_graph(builder, config, &connector)?;

    let domain_level_node_id = builder
        .make_all_aggregator(
            &[
                (
                    config_enabled_agg_id,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Normal,
                ),
                (
                    pms_enabled_agg_id,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Normal,
                ),
            ],
            Some(config_info),
            None::<()>,
            None,
        )
        .map_err(KgraphError::GraphConstructionError)?;
    let connector_dir_val = dir::DirValue::Connector(Box::new(ast::ConnectorChoice {
        connector,
        #[cfg(not(feature = "connector_choice_mca_id"))]
        sub_label: mca.business_sub_label.clone(),
    }));

    let connector_info = "Connector";
    let connector_node_id =
        builder.make_value_node(connector_dir_val.into(), Some(connector_info), None::<()>);

    builder
        .make_edge(
            domain_level_node_id,
            connector_node_id,
            cgraph::Strength::Normal,
            cgraph::Relation::Positive,
            None::<cgraph::DomainId>,
        )
        .map_err(KgraphError::GraphConstructionError)?;

    Ok(())
}

pub fn make_mca_graph<'a>(
    accts: Vec<admin_api::MerchantConnectorResponse>,
    config: &CountryCurrencyFilter,
) -> Result<cgraph::ConstraintGraph<'a, dir::DirValue>, KgraphError> {
    let mut builder = cgraph::ConstraintGraphBuilder::new();
    let _domain = builder.make_domain(
        DOMAIN_IDENTIFIER,
        "Payment methods enabled for MerchantConnectorAccount",
    );
    for acct in accts {
        compile_merchant_connector_graph(&mut builder, acct, config)?;
    }

    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::collections::{HashMap, HashSet};

    use api_models::enums as api_enums;
    use euclid::{
        dirval,
        dssa::graph::{AnalysisContext, CgraphExt},
    };
    use hyperswitch_constraint_graph::{ConstraintGraph, CycleCheck, Memoization};

    use super::*;
    use crate::utils::{NotAvailableFlows, PaymentMethodFilters};

    fn build_test_data<'a>() -> ConstraintGraph<'a, dir::DirValue> {
        use api_models::{admin::*, payment_methods::*};

        let stripe_account = MerchantConnectorResponse {
            connector_type: api_enums::ConnectorType::FizOperations,
            connector_name: "stripe".to_string(),
            merchant_connector_id: "something".to_string(),
            business_country: Some(api_enums::CountryAlpha2::US),
            connector_label: Some("something".to_string()),
            business_label: Some("food".to_string()),
            business_sub_label: None,
            connector_account_details: masking::Secret::new(serde_json::json!({})),
            test_mode: None,
            disabled: None,
            metadata: None,
            payment_methods_enabled: Some(vec![PaymentMethodsEnabled {
                payment_method: api_enums::PaymentMethod::Card,
                payment_method_types: Some(vec![
                    RequestPaymentMethodTypes {
                        payment_method_type: api_enums::PaymentMethodType::Credit,
                        payment_experience: None,
                        card_networks: Some(vec![
                            api_enums::CardNetwork::Visa,
                            api_enums::CardNetwork::Mastercard,
                        ]),
                        accepted_currencies: Some(AcceptedCurrencies::EnableOnly(vec![
                            api_enums::Currency::USD,
                            api_enums::Currency::INR,
                        ])),
                        accepted_countries: None,
                        minimum_amount: Some(10),
                        maximum_amount: Some(1000),
                        recurring_enabled: true,
                        installment_payment_enabled: true,
                    },
                    RequestPaymentMethodTypes {
                        payment_method_type: api_enums::PaymentMethodType::Debit,
                        payment_experience: None,
                        card_networks: Some(vec![
                            api_enums::CardNetwork::Maestro,
                            api_enums::CardNetwork::JCB,
                        ]),
                        accepted_currencies: Some(AcceptedCurrencies::EnableOnly(vec![
                            api_enums::Currency::GBP,
                            api_enums::Currency::PHP,
                        ])),
                        accepted_countries: None,
                        minimum_amount: Some(10),
                        maximum_amount: Some(1000),
                        recurring_enabled: true,
                        installment_payment_enabled: true,
                    },
                ]),
            }]),
            frm_configs: None,
            connector_webhook_details: None,
            profile_id: None,
            applepay_verified_domains: None,
            pm_auth_config: None,
            status: api_enums::ConnectorStatus::Inactive,
        };

        let currency_country_flow_filter = CurrencyCountryFlowFilter {
            currency: Some(HashSet::from([api_enums::Currency::INR])),
            country: Some(HashSet::from([api_enums::CountryAlpha2::IN])),
            not_available_flows: Some(NotAvailableFlows {
                capture_method: Some(api_enums::CaptureMethod::Manual),
            }),
        };
        let config_map = CountryCurrencyFilter {
            connector_configs: HashMap::from([(
                api_enums::RoutableConnectors::Stripe,
                PaymentMethodFilters(HashMap::from([(
                    PaymentMethodFilterKey::PaymentMethodType(api_enums::PaymentMethodType::Credit),
                    currency_country_flow_filter,
                )])),
            )]),
            default_configs: None,
        };
        // PaymentMethod(api_enums::PaymentMethodType::GooglePay),
        // currency_country_flow_filter,
        make_mca_graph(vec![stripe_account], &config_map).expect("Failed graph construction")
    }

    #[test]
    fn test_credit_card_success_case() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(PaymentMethod = Card),
                dirval!(CardType = Credit),
                dirval!(CardNetwork = Visa),
                dirval!(PaymentCurrency = INR),
                dirval!(PaymentAmount = 101),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_debit_card_success_case() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(PaymentMethod = Card),
                dirval!(CardType = Debit),
                dirval!(CardNetwork = Maestro),
                dirval!(PaymentCurrency = GBP),
                dirval!(PaymentAmount = 100),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_single_mismatch_failure_case() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(PaymentMethod = Card),
                dirval!(CardType = Debit),
                dirval!(CardNetwork = DinersClub),
                dirval!(PaymentCurrency = GBP),
                dirval!(PaymentAmount = 100),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_amount_mismatch_failure_case() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(PaymentMethod = Card),
                dirval!(CardType = Debit),
                dirval!(CardNetwork = Visa),
                dirval!(PaymentCurrency = GBP),
                dirval!(PaymentAmount = 7),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_incomplete_data_failure_case() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(PaymentMethod = Card),
                dirval!(CardType = Debit),
                dirval!(PaymentCurrency = GBP),
                dirval!(PaymentAmount = 7),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        //println!("{:#?}", result);
        //println!("{}", serde_json::to_string_pretty(&result).expect("Hello"));

        assert!(result.is_err());
    }

    #[test]
    fn test_incomplete_data_failure_case2() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(CardType = Debit),
                dirval!(CardNetwork = Visa),
                dirval!(PaymentCurrency = GBP),
                dirval!(PaymentAmount = 100),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        //println!("{:#?}", result);
        //println!("{}", serde_json::to_string_pretty(&result).expect("Hello"));

        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_applepay_bug_usecase() {
        let value = serde_json::json!([
            {
                "connector_type": "payment_processor",
                "connector_name": "bluesnap",
                "merchant_connector_id": "REDACTED",
                "status": "inactive",
                "connector_account_details": {
                    "auth_type": "BodyKey",
                    "api_key": "REDACTED",
                    "key1": "REDACTED"
                },
                "test_mode": true,
                "disabled": false,
                "payment_methods_enabled": [
                    {
                        "payment_method": "card",
                        "payment_method_types": [
                            {
                                "payment_method_type": "credit",
                                "payment_experience": null,
                                "card_networks": [
                                    "Mastercard",
                                    "Visa",
                                    "AmericanExpress",
                                    "JCB",
                                    "DinersClub",
                                    "Discover",
                                    "CartesBancaires",
                                    "UnionPay"
                                ],
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            },
                            {
                                "payment_method_type": "debit",
                                "payment_experience": null,
                                "card_networks": [
                                    "Mastercard",
                                    "Visa",
                                    "Interac",
                                    "AmericanExpress",
                                    "JCB",
                                    "DinersClub",
                                    "Discover",
                                    "CartesBancaires",
                                    "UnionPay"
                                ],
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            }
                        ]
                    },
                    {
                        "payment_method": "wallet",
                        "payment_method_types": [
                            {
                                "payment_method_type": "google_pay",
                                "payment_experience": "invoke_sdk_client",
                                "card_networks": null,
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            }
                        ]
                    }
                ],
                "metadata": {},
                "business_country": "US",
                "business_label": "default",
                "business_sub_label": null,
                "frm_configs": null
            },
            {
                "connector_type": "payment_processor",
                "connector_name": "stripe",
                "merchant_connector_id": "REDACTED",
                "status": "inactive",
                "connector_account_details": {
                    "auth_type": "HeaderKey",
                    "api_key": "REDACTED"
                },
                "test_mode": true,
                "disabled": false,
                "payment_methods_enabled": [
                    {
                        "payment_method": "card",
                        "payment_method_types": [
                            {
                                "payment_method_type": "credit",
                                "payment_experience": null,
                                "card_networks": [
                                    "Mastercard",
                                    "Visa",
                                    "AmericanExpress",
                                    "JCB",
                                    "DinersClub",
                                    "Discover",
                                    "CartesBancaires",
                                    "UnionPay"
                                ],
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            },
                            {
                                "payment_method_type": "debit",
                                "payment_experience": null,
                                "card_networks": [
                                    "Mastercard",
                                    "Visa",
                                    "Interac",
                                    "AmericanExpress",
                                    "JCB",
                                    "DinersClub",
                                    "Discover",
                                    "CartesBancaires",
                                    "UnionPay"
                                ],
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            }
                        ]
                    },
                    {
                        "payment_method": "wallet",
                        "payment_method_types": [
                            {
                                "payment_method_type": "apple_pay",
                                "payment_experience": "invoke_sdk_client",
                                "card_networks": null,
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            }
                        ]
                    },
                    {
                        "payment_method": "pay_later",
                        "payment_method_types": []
                    }
                ],
                "metadata": {},
                "business_country": "US",
                "business_label": "default",
                "business_sub_label": null,
                "frm_configs": null
            }
        ]);

        let data: Vec<admin_api::MerchantConnectorResponse> =
            serde_json::from_value(value).expect("data");
        let config = CountryCurrencyFilter {
            connector_configs: HashMap::new(),
            default_configs: None,
        };
        let graph = make_mca_graph(data, &config).expect("graph");
        let context = AnalysisContext::from_dir_values([
            dirval!(Connector = Stripe),
            dirval!(PaymentAmount = 212),
            dirval!(PaymentCurrency = ILS),
            dirval!(PaymentMethod = Wallet),
            dirval!(WalletType = ApplePay),
        ]);

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &context,
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok(), "stripe validation failed");

        let result = graph.key_value_analysis(
            dirval!(Connector = Bluesnap),
            &context,
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );
        assert!(result.is_err(), "bluesnap validation failed");
    }
}
