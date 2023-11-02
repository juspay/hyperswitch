#![allow(non_upper_case_globals)]
mod types;
mod utils;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use api_models::{admin as admin_api, routing::ConnectorSelection};
use euclid::{
    backend::{inputs, interpreter::InterpreterBackend, EuclidBackend},
    dssa::{
        self, analyzer,
        graph::{self, Memoization},
        state_machine, truth,
    },
    enums,
    frontend::{
        ast,
        dir::{self, enums as dir_enums},
    },
};
use once_cell::sync::OnceCell;
use strum::{EnumMessage, EnumProperty, VariantNames};
use wasm_bindgen::prelude::*;

use crate::utils::JsResultExt;
type JsResult = Result<JsValue, JsValue>;

struct SeedData<'a> {
    kgraph: graph::KnowledgeGraph<'a>,
    connectors: Vec<ast::ConnectorChoice>,
}

static SEED_DATA: OnceCell<SeedData<'_>> = OnceCell::new();

/// This function can be used by the frontend to provide the WASM with information about
/// all the merchant's connector accounts. The input argument is a vector of all the merchant's
/// connector accounts from the API.
#[wasm_bindgen(js_name = seedKnowledgeGraph)]
pub fn seed_knowledge_graph(mcas: JsValue) -> JsResult {
    let mcas: Vec<admin_api::MerchantConnectorResponse> = serde_wasm_bindgen::from_value(mcas)?;
    let connectors: Vec<ast::ConnectorChoice> = mcas
        .iter()
        .map(|mca| {
            Ok::<_, strum::ParseError>(ast::ConnectorChoice {
                connector: dir_enums::Connector::from_str(&mca.connector_name)?,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                sub_label: mca.business_sub_label.clone(),
            })
        })
        .collect::<Result<_, _>>()
        .map_err(|_| "invalid connector name received")
        .err_to_js()?;

    let mca_graph = kgraph_utils::mca::make_mca_graph(mcas).err_to_js()?;
    let analysis_graph =
        graph::KnowledgeGraph::combine(&mca_graph, &truth::ANALYSIS_GRAPH).err_to_js()?;

    SEED_DATA
        .set(SeedData {
            kgraph: analysis_graph,
            connectors,
        })
        .map_err(|_| "Knowledge Graph has been already seeded".to_string())
        .err_to_js()?;

    Ok(JsValue::NULL)
}

/// This function allows the frontend to get all the merchant's configured
/// connectors that are valid for a rule based on the conditions specified in
/// the rule
#[wasm_bindgen(js_name = getValidConnectorsForRule)]
pub fn get_valid_connectors_for_rule(rule: JsValue) -> JsResult {
    let seed_data = SEED_DATA.get().ok_or("Data not seeded").err_to_js()?;

    let rule: ast::Rule<ConnectorSelection> = serde_wasm_bindgen::from_value(rule)?;
    let dir_rule = ast::lowering::lower_rule(rule).err_to_js()?;
    let mut valid_connectors: Vec<(ast::ConnectorChoice, dir::DirValue)> = seed_data
        .connectors
        .iter()
        .cloned()
        .map(|choice| (choice.clone(), dir::DirValue::Connector(Box::new(choice))))
        .collect();
    let mut invalid_connectors: HashSet<ast::ConnectorChoice> = HashSet::new();

    let mut ctx_manager = state_machine::RuleContextManager::new(&dir_rule, &[]);

    let dummy_meta = HashMap::new();

    // For every conjunctive context in the Rule, verify validity of all still-valid connectors
    // using the knowledge graph
    while let Some(ctx) = ctx_manager.advance_mut().err_to_js()? {
        // Standalone conjunctive context analysis to ensure the context itself is valid before
        // checking it against merchant's connectors
        seed_data
            .kgraph
            .perform_context_analysis(ctx, &mut Memoization::new())
            .err_to_js()?;

        // Update conjunctive context and run analysis on all of merchant's connectors.
        for (conn, choice) in &valid_connectors {
            if invalid_connectors.contains(conn) {
                continue;
            }

            let ctx_val = dssa::types::ContextValue::assertion(choice, &dummy_meta);
            ctx.push(ctx_val);
            let analysis_result = seed_data
                .kgraph
                .perform_context_analysis(ctx, &mut Memoization::new());
            if analysis_result.is_err() {
                invalid_connectors.insert(conn.clone());
            }
            ctx.pop();
        }
    }

    valid_connectors.retain(|(k, _)| !invalid_connectors.contains(k));

    let valid_connectors: Vec<ast::ConnectorChoice> =
        valid_connectors.into_iter().map(|c| c.0).collect();

    Ok(serde_wasm_bindgen::to_value(&valid_connectors)?)
}

#[wasm_bindgen(js_name = analyzeProgram)]
pub fn analyze_program(js_program: JsValue) -> JsResult {
    let program: ast::Program<ConnectorSelection> = serde_wasm_bindgen::from_value(js_program)?;
    analyzer::analyze(program, SEED_DATA.get().map(|sd| &sd.kgraph)).err_to_js()?;
    Ok(JsValue::NULL)
}

#[wasm_bindgen(js_name = runProgram)]
pub fn run_program(program: JsValue, input: JsValue) -> JsResult {
    let program: ast::Program<ConnectorSelection> = serde_wasm_bindgen::from_value(program)?;
    let input: inputs::BackendInput = serde_wasm_bindgen::from_value(input)?;

    let backend = InterpreterBackend::with_program(program).err_to_js()?;

    let res: euclid::backend::BackendOutput<ConnectorSelection> =
        backend.execute(input).err_to_js()?;

    Ok(serde_wasm_bindgen::to_value(&res)?)
}

#[wasm_bindgen(js_name = getAllConnectors)]
pub fn get_all_connectors() -> JsResult {
    Ok(serde_wasm_bindgen::to_value(enums::Connector::VARIANTS)?)
}

#[wasm_bindgen(js_name = getAllKeys)]
pub fn get_all_keys() -> JsResult {
    let keys: Vec<&'static str> = dir::DirKeyKind::VARIANTS
        .iter()
        .copied()
        .filter(|s| s != &"Connector")
        .collect();
    Ok(serde_wasm_bindgen::to_value(&keys)?)
}

#[wasm_bindgen(js_name = getKeyType)]
pub fn get_key_type(key: &str) -> Result<String, String> {
    let key = dir::DirKeyKind::from_str(key).map_err(|_| "Invalid key received".to_string())?;
    let key_str = key.get_type().to_string();
    Ok(key_str)
}

#[wasm_bindgen(js_name=parseToString)]
pub fn parser(val: String) -> String {
    ron_parser::my_parse(val)
}

#[wasm_bindgen(js_name = getVariantValues)]
pub fn get_variant_values(key: &str) -> Result<JsValue, JsValue> {
    let key = dir::DirKeyKind::from_str(key).map_err(|_| "Invalid key received".to_string())?;

    let variants: &[&str] = match key {
        dir::DirKeyKind::PaymentMethod => dir_enums::PaymentMethod::VARIANTS,
        dir::DirKeyKind::CardType => dir_enums::CardType::VARIANTS,
        dir::DirKeyKind::CardNetwork => dir_enums::CardNetwork::VARIANTS,
        dir::DirKeyKind::PayLaterType => dir_enums::PayLaterType::VARIANTS,
        dir::DirKeyKind::WalletType => dir_enums::WalletType::VARIANTS,
        dir::DirKeyKind::BankRedirectType => dir_enums::BankRedirectType::VARIANTS,
        dir::DirKeyKind::CryptoType => dir_enums::CryptoType::VARIANTS,
        dir::DirKeyKind::RewardType => dir_enums::RewardType::VARIANTS,
        dir::DirKeyKind::AuthenticationType => dir_enums::AuthenticationType::VARIANTS,
        dir::DirKeyKind::CaptureMethod => dir_enums::CaptureMethod::VARIANTS,
        dir::DirKeyKind::PaymentCurrency => dir_enums::PaymentCurrency::VARIANTS,
        dir::DirKeyKind::BusinessCountry => dir_enums::Country::VARIANTS,
        dir::DirKeyKind::BillingCountry => dir_enums::Country::VARIANTS,
        dir::DirKeyKind::BankTransferType => dir_enums::BankTransferType::VARIANTS,
        dir::DirKeyKind::UpiType => dir_enums::UpiType::VARIANTS,
        dir::DirKeyKind::SetupFutureUsage => dir_enums::SetupFutureUsage::VARIANTS,
        dir::DirKeyKind::PaymentType => dir_enums::PaymentType::VARIANTS,
        dir::DirKeyKind::MandateType => dir_enums::MandateType::VARIANTS,
        dir::DirKeyKind::MandateAcceptanceType => dir_enums::MandateAcceptanceType::VARIANTS,
        dir::DirKeyKind::CardRedirectType => dir_enums::CardRedirectType::VARIANTS,
        dir::DirKeyKind::GiftCardType => dir_enums::GiftCardType::VARIANTS,
        dir::DirKeyKind::VoucherType => dir_enums::VoucherType::VARIANTS,
        dir::DirKeyKind::PaymentAmount
        | dir::DirKeyKind::Connector
        | dir::DirKeyKind::CardBin
        | dir::DirKeyKind::BusinessLabel
        | dir::DirKeyKind::MetaData => Err("Key does not have variants".to_string())?,
        dir::DirKeyKind::BankDebitType => dir_enums::BankDebitType::VARIANTS,
    };

    Ok(serde_wasm_bindgen::to_value(variants)?)
}

#[wasm_bindgen(js_name = addTwo)]
pub fn add_two(n1: i64, n2: i64) -> i64 {
    n1 + n2
}

#[wasm_bindgen(js_name = getDescriptionCategory)]
pub fn get_description_category(key: &str) -> JsResult {
    let key = dir::DirKeyKind::from_str(key).map_err(|_| "Invalid key received".to_string())?;

    let result = types::Details {
        description: key.get_detailed_message(),
        category: key.get_str("Category"),
    };
    Ok(serde_wasm_bindgen::to_value(&result)?)
}
