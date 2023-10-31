use std::str::FromStr;

use api_models::{
    admin as admin_api, enums as api_enums, payment_methods::RequestPaymentMethodTypes,
};
use euclid::{
    dssa::graph::{self, DomainIdentifier},
    frontend::{
        ast,
        dir::{self, enums as dir_enums},
    },
    types::{NumValue, NumValueRefinement},
};

use crate::{error::KgraphError, transformers::IntoDirValue};

pub const DOMAIN_IDENTIFIER: &str = "payment_methods_enabled_for_merchantconnectoraccount";

fn compile_request_pm_types(
    builder: &mut graph::KnowledgeGraphBuilder<'_>,
    pm_types: RequestPaymentMethodTypes,
    pm: api_enums::PaymentMethod,
) -> Result<graph::NodeId, KgraphError> {
    let mut agg_nodes: Vec<(graph::NodeId, graph::Relation, graph::Strength)> = Vec::new();

    let pmt_info = "PaymentMethodType";
    let pmt_id = builder
        .make_value_node(
            (pm_types.payment_method_type, pm)
                .into_dir_value()
                .map(Into::into)?,
            Some(pmt_info),
            vec![DomainIdentifier::new(DOMAIN_IDENTIFIER)],
            None::<()>,
        )
        .map_err(KgraphError::GraphConstructionError)?;
    agg_nodes.push((
        pmt_id,
        graph::Relation::Positive,
        match pm_types.payment_method_type {
            api_enums::PaymentMethodType::Credit | api_enums::PaymentMethodType::Debit => {
                graph::Strength::Weak
            }

            _ => graph::Strength::Strong,
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
                .make_in_aggregator(dir_vals, Some(card_network_info), None::<()>, Vec::new())
                .map_err(KgraphError::GraphConstructionError)?;

            agg_nodes.push((
                card_network_id,
                graph::Relation::Positive,
                graph::Strength::Weak,
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
                graph::Relation::Positive,
            )),

            admin_api::AcceptedCurrencies::DisableOnly(curr) if !curr.is_empty() => Some((
                curr.into_iter()
                    .map(IntoDirValue::into_dir_value)
                    .collect::<Result<_, _>>()
                    .ok()?,
                graph::Relation::Negative,
            )),

            _ => None,
        });

    if let Some((currencies, relation)) = currencies_data {
        let accepted_currencies_info = "Accepted Currencies";
        let accepted_currencies_id = builder
            .make_in_aggregator(
                currencies,
                Some(accepted_currencies_info),
                None::<()>,
                Vec::new(),
            )
            .map_err(KgraphError::GraphConstructionError)?;

        agg_nodes.push((accepted_currencies_id, relation, graph::Strength::Strong));
    }

    let mut amount_nodes = Vec::with_capacity(2);

    if let Some(min_amt) = pm_types.minimum_amount {
        let num_val = NumValue {
            number: min_amt.into(),
            refinement: Some(NumValueRefinement::GreaterThanEqual),
        };

        let min_amt_info = "Minimum Amount";
        let min_amt_id = builder
            .make_value_node(
                dir::DirValue::PaymentAmount(num_val).into(),
                Some(min_amt_info),
                vec![DomainIdentifier::new(DOMAIN_IDENTIFIER)],
                None::<()>,
            )
            .map_err(KgraphError::GraphConstructionError)?;

        amount_nodes.push(min_amt_id);
    }

    if let Some(max_amt) = pm_types.maximum_amount {
        let num_val = NumValue {
            number: max_amt.into(),
            refinement: Some(NumValueRefinement::LessThanEqual),
        };

        let max_amt_info = "Maximum Amount";
        let max_amt_id = builder
            .make_value_node(
                dir::DirValue::PaymentAmount(num_val).into(),
                Some(max_amt_info),
                vec![DomainIdentifier::new(DOMAIN_IDENTIFIER)],
                None::<()>,
            )
            .map_err(KgraphError::GraphConstructionError)?;

        amount_nodes.push(max_amt_id);
    }

    if !amount_nodes.is_empty() {
        let zero_num_val = NumValue {
            number: 0,
            refinement: None,
        };

        let zero_amt_id = builder
            .make_value_node(
                dir::DirValue::PaymentAmount(zero_num_val).into(),
                Some("zero_amount"),
                vec![DomainIdentifier::new(DOMAIN_IDENTIFIER)],
                None::<()>,
            )
            .map_err(KgraphError::GraphConstructionError)?;

        let or_node_neighbor_id = if amount_nodes.len() == 1 {
            amount_nodes
                .get(0)
                .copied()
                .ok_or(KgraphError::IndexingError)?
        } else {
            let nodes = amount_nodes
                .iter()
                .copied()
                .map(|node_id| (node_id, graph::Relation::Positive, graph::Strength::Strong))
                .collect::<Vec<_>>();

            builder
                .make_all_aggregator(
                    &nodes,
                    Some("amount_constraint_aggregator"),
                    None::<()>,
                    vec![DomainIdentifier::new(DOMAIN_IDENTIFIER)],
                )
                .map_err(KgraphError::GraphConstructionError)?
        };

        let any_aggregator = builder
            .make_any_aggregator(
                &[
                    (zero_amt_id, graph::Relation::Positive),
                    (or_node_neighbor_id, graph::Relation::Positive),
                ],
                Some("zero_plus_limits_amount_aggregator"),
                None::<()>,
                vec![DomainIdentifier::new(DOMAIN_IDENTIFIER)],
            )
            .map_err(KgraphError::GraphConstructionError)?;

        agg_nodes.push((
            any_aggregator,
            graph::Relation::Positive,
            graph::Strength::Strong,
        ));
    }

    let pmt_all_aggregator_info = "All Aggregator for PaymentMethodType";
    builder
        .make_all_aggregator(
            &agg_nodes,
            Some(pmt_all_aggregator_info),
            None::<()>,
            Vec::new(),
        )
        .map_err(KgraphError::GraphConstructionError)
}

fn compile_payment_method_enabled(
    builder: &mut graph::KnowledgeGraphBuilder<'_>,
    enabled: admin_api::PaymentMethodsEnabled,
) -> Result<Option<graph::NodeId>, KgraphError> {
    let agg_id = if !enabled
        .payment_method_types
        .as_ref()
        .map(|v| v.is_empty())
        .unwrap_or(true)
    {
        let pm_info = "PaymentMethod";
        let pm_id = builder
            .make_value_node(
                enabled.payment_method.into_dir_value().map(Into::into)?,
                Some(pm_info),
                vec![DomainIdentifier::new(DOMAIN_IDENTIFIER)],
                None::<()>,
            )
            .map_err(KgraphError::GraphConstructionError)?;

        let mut agg_nodes: Vec<(graph::NodeId, graph::Relation)> = Vec::new();

        if let Some(pm_types) = enabled.payment_method_types {
            for pm_type in pm_types {
                let node_id = compile_request_pm_types(builder, pm_type, enabled.payment_method)?;
                agg_nodes.push((node_id, graph::Relation::Positive));
            }
        }

        let any_aggregator_info = "Any aggregation for PaymentMethodsType";
        let pm_type_agg_id = builder
            .make_any_aggregator(
                &agg_nodes,
                Some(any_aggregator_info),
                None::<()>,
                Vec::new(),
            )
            .map_err(KgraphError::GraphConstructionError)?;

        let all_aggregator_info = "All aggregation for PaymentMethod";
        let enabled_pm_agg_id = builder
            .make_all_aggregator(
                &[
                    (pm_id, graph::Relation::Positive, graph::Strength::Strong),
                    (
                        pm_type_agg_id,
                        graph::Relation::Positive,
                        graph::Strength::Strong,
                    ),
                ],
                Some(all_aggregator_info),
                None::<()>,
                Vec::new(),
            )
            .map_err(KgraphError::GraphConstructionError)?;

        Some(enabled_pm_agg_id)
    } else {
        None
    };

    Ok(agg_id)
}

fn compile_merchant_connector_graph(
    builder: &mut graph::KnowledgeGraphBuilder<'_>,
    mca: admin_api::MerchantConnectorResponse,
) -> Result<(), KgraphError> {
    let connector = dir_enums::Connector::from_str(&mca.connector_name)
        .map_err(|_| KgraphError::InvalidConnectorName(mca.connector_name.clone()))?;

    let mut agg_nodes: Vec<(graph::NodeId, graph::Relation)> = Vec::new();

    if let Some(pms_enabled) = mca.payment_methods_enabled {
        for pm_enabled in pms_enabled {
            let maybe_pm_enabled_id = compile_payment_method_enabled(builder, pm_enabled)?;
            if let Some(pm_enabled_id) = maybe_pm_enabled_id {
                agg_nodes.push((pm_enabled_id, graph::Relation::Positive));
            }
        }
    }

    let aggregator_info = "Available Payment methods for connector";
    let pms_enabled_agg_id = builder
        .make_any_aggregator(&agg_nodes, Some(aggregator_info), None::<()>, Vec::new())
        .map_err(KgraphError::GraphConstructionError)?;

    let connector_dir_val = dir::DirValue::Connector(Box::new(ast::ConnectorChoice {
        connector,
        #[cfg(not(feature = "connector_choice_mca_id"))]
        sub_label: mca.business_sub_label,
    }));

    let connector_info = "Connector";
    let connector_node_id = builder
        .make_value_node(
            connector_dir_val.into(),
            Some(connector_info),
            vec![DomainIdentifier::new(DOMAIN_IDENTIFIER)],
            None::<()>,
        )
        .map_err(KgraphError::GraphConstructionError)?;

    builder
        .make_edge(
            pms_enabled_agg_id,
            connector_node_id,
            graph::Strength::Normal,
            graph::Relation::Positive,
        )
        .map_err(KgraphError::GraphConstructionError)?;

    Ok(())
}

pub fn make_mca_graph<'a>(
    accts: Vec<admin_api::MerchantConnectorResponse>,
) -> Result<graph::KnowledgeGraph<'a>, KgraphError> {
    let mut builder = graph::KnowledgeGraphBuilder::new();
    let _domain = builder.make_domain(
        DomainIdentifier::new(DOMAIN_IDENTIFIER),
        "Payment methods enabled for MerchantConnectorAccount".to_string(),
    );
    for acct in accts {
        compile_merchant_connector_graph(&mut builder, acct)?;
    }

    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use api_models::enums as api_enums;
    use euclid::{
        dirval,
        dssa::graph::{AnalysisContext, Memoization},
    };

    use super::*;

    fn build_test_data<'a>() -> graph::KnowledgeGraph<'a> {
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
        };

        make_mca_graph(vec![stripe_account]).expect("Failed graph construction")
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
                dirval!(PaymentCurrency = USD),
                dirval!(PaymentAmount = 100),
            ]),
            &mut Memoization::new(),
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

        let graph = make_mca_graph(data).expect("graph");
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
        );

        assert!(result.is_ok(), "stripe validation failed");

        let result = graph.key_value_analysis(
            dirval!(Connector = Bluesnap),
            &context,
            &mut Memoization::new(),
        );
        assert!(result.is_err(), "bluesnap validation failed");
    }
}
