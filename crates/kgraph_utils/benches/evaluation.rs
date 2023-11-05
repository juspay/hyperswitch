#![allow(unused, clippy::expect_used)]

use std::str::FromStr;

use api_models::{
    admin as admin_api, enums as api_enums, payment_methods::RequestPaymentMethodTypes,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use euclid::{
    dirval,
    dssa::graph::{self, Memoization},
    frontend::dir,
    types::{NumValue, NumValueRefinement},
};
use kgraph_utils::{error::KgraphError, transformers::IntoDirValue};

fn build_test_data<'a>(total_enabled: usize, total_pm_types: usize) -> graph::KnowledgeGraph<'a> {
    use api_models::{admin::*, payment_methods::*};

    let mut pms_enabled: Vec<PaymentMethodsEnabled> = Vec::new();

    for _ in (0..total_enabled) {
        let mut pm_types: Vec<RequestPaymentMethodTypes> = Vec::new();
        for _ in (0..total_pm_types) {
            pm_types.push(RequestPaymentMethodTypes {
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
            });
        }

        pms_enabled.push(PaymentMethodsEnabled {
            payment_method: api_enums::PaymentMethod::Card,
            payment_method_types: Some(pm_types),
        });
    }

    let stripe_account = MerchantConnectorResponse {
        connector_type: api_enums::ConnectorType::FizOperations,
        connector_name: "stripe".to_string(),
        merchant_connector_id: "something".to_string(),
        connector_account_details: masking::Secret::new(serde_json::json!({})),
        test_mode: None,
        disabled: None,
        metadata: None,
        payment_methods_enabled: Some(pms_enabled),
        business_country: Some(api_enums::CountryAlpha2::US),
        business_label: Some("hello".to_string()),
        connector_label: Some("something".to_string()),
        business_sub_label: Some("something".to_string()),
        frm_configs: None,
        connector_webhook_details: None,
        profile_id: None,
        applepay_verified_domains: None,
        pm_auth_config: None,
    };

    kgraph_utils::mca::make_mca_graph(vec![stripe_account]).expect("Failed graph construction")
}

fn evaluation(c: &mut Criterion) {
    let small_graph = build_test_data(3, 8);
    let big_graph = build_test_data(20, 20);

    c.bench_function("MCA Small Graph Evaluation", |b| {
        b.iter(|| {
            small_graph.key_value_analysis(
                dirval!(Connector = Stripe),
                &graph::AnalysisContext::from_dir_values([
                    dirval!(Connector = Stripe),
                    dirval!(PaymentMethod = Card),
                    dirval!(CardType = Credit),
                    dirval!(CardNetwork = Visa),
                    dirval!(PaymentCurrency = BWP),
                    dirval!(PaymentAmount = 100),
                ]),
                &mut Memoization::new(),
            );
        });
    });

    c.bench_function("MCA Big Graph Evaluation", |b| {
        b.iter(|| {
            big_graph.key_value_analysis(
                dirval!(Connector = Stripe),
                &graph::AnalysisContext::from_dir_values([
                    dirval!(Connector = Stripe),
                    dirval!(PaymentMethod = Card),
                    dirval!(CardType = Credit),
                    dirval!(CardNetwork = Visa),
                    dirval!(PaymentCurrency = BWP),
                    dirval!(PaymentAmount = 100),
                ]),
                &mut Memoization::new(),
            );
        });
    });
}

criterion_group!(benches, evaluation);
criterion_main!(benches);
