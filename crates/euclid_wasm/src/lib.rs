#![allow(non_upper_case_globals)]
mod types;
mod utils;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use api_models::{
    admin as admin_api, conditional_configs::ConditionalConfigs, enums as api_model_enums,
    routing::ConnectorSelection, surcharge_decision_configs::SurchargeDecisionConfigs,
};
use common_enums::RoutableConnectors;
use connector_configs::{
    common_config::{ConnectorApiIntegrationPayload, DashboardRequestPayload},
    connector,
};
use currency_conversion::{
    conversion::convert as convert_currency, types as currency_conversion_types,
};
use euclid::{
    backend::{inputs, interpreter::InterpreterBackend, EuclidBackend},
    dssa::{
        self, analyzer,
        graph::{self, Memoization},
        state_machine, truth,
    },
    frontend::{
        ast,
        dir::{self, enums as dir_enums, EuclidDirFilter},
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
static SEED_FOREX: OnceCell<currency_conversion_types::ExchangeRates> = OnceCell::new();

/// This function can be used by the frontend to educate wasm about the forex rates data.
/// The input argument is a struct fields base_currency and conversion where later is all the conversions associated with the base_currency
/// to all different currencies present.
#[wasm_bindgen(js_name = setForexData)]
/// Extracts and seeds forex exchange rates into the application. If the forex exchange rates have already been seeded, it returns an error message in JavaScript.
pub fn seed_forex(forex: JsValue) -> JsResult {
    let forex: currency_conversion_types::ExchangeRates = serde_wasm_bindgen::from_value(forex)?;
    SEED_FOREX
        .set(forex)
        .map_err(|_| "Forex has already been seeded".to_string())
        .err_to_js()?;

    Ok(JsValue::NULL)
}

/// This function can be used to perform currency_conversion on the input amount, from_currency,
/// to_currency which are all expected to be one of currencies we already have in our Currency
/// enum.
#[wasm_bindgen(js_name = convertCurrency)]
/// Convert the given amount from one currency to another using the provided forex data.
///
/// # Arguments
///
/// * `amount` - The amount to be converted
/// * `from_currency` - The currency from which the amount will be converted
/// * `to_currency` - The currency to which the amount will be converted
///
/// # Returns
///
/// The converted amount in the specified currency, or an error if the conversion is not possible.
pub fn convert_forex_value(amount: i64, from_currency: JsValue, to_currency: JsValue) -> JsResult {
    let forex_data = SEED_FOREX
        .get()
        .ok_or("Forex Data not seeded")
        .err_to_js()?;
    let from_currency: common_enums::Currency = serde_wasm_bindgen::from_value(from_currency)?;
    let to_currency: common_enums::Currency = serde_wasm_bindgen::from_value(to_currency)?;
    let converted_amount = convert_currency(forex_data, from_currency, to_currency, amount)
        .map_err(|_| "conversion not possible for provided values")
        .err_to_js()?;

    Ok(serde_wasm_bindgen::to_value(&converted_amount)?)
}

/// This function can be used by the frontend to provide the WASM with information about
/// all the merchant's connector accounts. The input argument is a vector of all the merchant's
/// connector accounts from the API.
#[wasm_bindgen(js_name = seedKnowledgeGraph)]
/// Seed the knowledge graph with the provided merchant connector responses.
pub fn seed_knowledge_graph(mcas: JsValue) -> JsResult {
    let mcas: Vec<admin_api::MerchantConnectorResponse> = serde_wasm_bindgen::from_value(mcas)?;
    let connectors: Vec<ast::ConnectorChoice> = mcas
        .iter()
        .map(|mca| {
            Ok::<_, strum::ParseError>(ast::ConnectorChoice {
                connector: RoutableConnectors::from_str(&mca.connector_name)?,
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
/// Retrieves the valid connectors for a given rule by performing context analysis on the knowledge graph.
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
/// Analyzes a JavaScript program by deserializing it into an abstract syntax tree representation, then performs analysis using the provided seed data and returns the result as a JavaScript value.
pub fn analyze_program(js_program: JsValue) -> JsResult {
    let program: ast::Program<ConnectorSelection> = serde_wasm_bindgen::from_value(js_program)?;
    analyzer::analyze(program, SEED_DATA.get().map(|sd| &sd.kgraph)).err_to_js()?;
    Ok(JsValue::NULL)
}

#[wasm_bindgen(js_name = runProgram)]
/// Runs the provided program using the given input and returns the result.
pub fn run_program(program: JsValue, input: JsValue) -> JsResult {
    let program: ast::Program<ConnectorSelection> = serde_wasm_bindgen::from_value(program)?;
    let input: inputs::BackendInput = serde_wasm_bindgen::from_value(input)?;

    let backend = InterpreterBackend::with_program(program).err_to_js()?;

    let res: euclid::backend::BackendOutput<ConnectorSelection> =
        backend.execute(input).err_to_js()?;

    Ok(serde_wasm_bindgen::to_value(&res)?)
}

#[wasm_bindgen(js_name = getAllConnectors)]
/// Retrieves all available connectors and returns them as a JsResult.
pub fn get_all_connectors() -> JsResult {
    Ok(serde_wasm_bindgen::to_value(
        common_enums::RoutableConnectors::VARIANTS,
    )?)
}

#[wasm_bindgen(js_name = getAllKeys)]
/// Retrieves all keys from the DirKeyKind enum, excluding the "Connector" variant, and returns them as a JavaScript Promise.
pub fn get_all_keys() -> JsResult {
    let keys: Vec<&'static str> = dir::DirKeyKind::VARIANTS
        .iter()
        .copied()
        .filter(|s| s != &"Connector")
        .collect();
    Ok(serde_wasm_bindgen::to_value(&keys)?)
}

#[wasm_bindgen(js_name = getKeyType)]
/// Retrieves the type of a given key and returns it as a Result. If the key is invalid, an error message is returned.
pub fn get_key_type(key: &str) -> Result<String, String> {
    let key = dir::DirKeyKind::from_str(key).map_err(|_| "Invalid key received".to_string())?;
    let key_str = key.get_type().to_string();
    Ok(key_str)
}

#[wasm_bindgen(js_name = getThreeDsKeys)]
/// Retrieves the three dimensional keys from the ConditionalConfigs as EuclidDirFilter and returns them as a JsResult.
pub fn get_three_ds_keys() -> JsResult {
    let keys = <ConditionalConfigs as EuclidDirFilter>::ALLOWED;
    Ok(serde_wasm_bindgen::to_value(keys)?)
}

#[wasm_bindgen(js_name= getSurchargeKeys)]
/// Retrieves the allowed surcharge keys from the SurchargeDecisionConfigs as EuclidDirFilter and returns them as a JsResult.
pub fn get_surcharge_keys() -> JsResult {
    let keys = <SurchargeDecisionConfigs as EuclidDirFilter>::ALLOWED;
    Ok(serde_wasm_bindgen::to_value(keys)?)
}

#[wasm_bindgen(js_name=parseToString)]
/// Parses the given string using the `my_parse` function from the `ron_parser` module.
///
/// # Arguments
///
/// * `val` - A string to be parsed
///
/// # Returns
///
/// A string containing the result of parsing the input string
pub fn parser(val: String) -> String {
    ron_parser::my_parse(val)
}

#[wasm_bindgen(js_name = getVariantValues)]
/// Retrieves the list of variants for a given key from the directory enums. Returns a Result containing a JsValue with the variant values if successful, or a JsValue with an error message if the key is invalid or does not have any variants.
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
/// Adds two numbers together and returns the result.
pub fn add_two(n1: i64, n2: i64) -> i64 {
    n1 + n2
}

#[wasm_bindgen(js_name = getDescriptionCategory)]
/// This method retrieves the description category by iterating through the variants of DirKeyKind. It filters out the "Connector" variant, and then creates a HashMap where the key is an Option<&str> and the value is a Vec of types::Details. It populates the HashMap by creating details for each key, getting the detailed message from the dir_key, and then organizing the details by the "Category" received from the dir_key. Finally, it returns the serialized value of the category as a JsValue.
pub fn get_description_category() -> JsResult {
    let keys = dir::DirKeyKind::VARIANTS
        .iter()
        .copied()
        .filter(|s| s != &"Connector")
        .collect::<Vec<&'static str>>();
    let mut category: HashMap<Option<&str>, Vec<types::Details<'_>>> = HashMap::new();
    for key in keys {
        let dir_key =
            dir::DirKeyKind::from_str(key).map_err(|_| "Invalid key received".to_string())?;
        let details = types::Details {
            description: dir_key.get_detailed_message(),
            kind: dir_key.clone(),
        };
        category
            .entry(dir_key.get_str("Category"))
            .and_modify(|val| val.push(details.clone()))
            .or_insert(vec![details]);
    }

    Ok(serde_wasm_bindgen::to_value(&category)?)
}

#[wasm_bindgen(js_name = getConnectorConfig)]
/// Retrieves the connector configuration for the given key.
///
/// # Arguments
///
/// * `key` - A string reference representing the key for the connector configuration.
///
/// # Returns
///
/// * `JsResult` - A result containing the connector configuration if successful, or an error message if the key is invalid or the configuration retrieval fails.
///
pub fn get_connector_config(key: &str) -> JsResult {
    let key = api_model_enums::Connector::from_str(key)
        .map_err(|_| "Invalid key received".to_string())?;
    let res = connector::ConnectorConfig::get_connector_config(key)?;
    Ok(serde_wasm_bindgen::to_value(&res)?)
}

#[cfg(feature = "payouts")]
#[wasm_bindgen(js_name = getPayoutConnectorConfig)]
/// Retrieves the payout connector configuration based on the given key.
///
/// # Arguments
///
/// * `key` - A string reference representing the key for the payout connector configuration.
///
/// # Returns
///
/// Returns a `JsResult` containing the payout connector configuration if the key is valid, otherwise an error message.
///
/// # Errors
///
/// Returns an error if the provided key is invalid or if there is an issue retrieving the payout connector configuration.
///
pub fn get_payout_connector_config(key: &str) -> JsResult {
    let key = api_model_enums::PayoutConnectors::from_str(key)
        .map_err(|_| "Invalid key received".to_string())?;
    let res = connector::ConnectorConfig::get_payout_connector_config(key)?;
    Ok(serde_wasm_bindgen::to_value(&res)?)
}

#[wasm_bindgen(js_name = getRequestPayload)]
/// Converts the input JsValue into a DashboardRequestPayload and the response JsValue into a ConnectorApiIntegrationPayload. Then creates a connector request using the input payload and the api response payload. Finally, it converts the result into a JsValue and returns it as a JsResult.
pub fn get_request_payload(input: JsValue, response: JsValue) -> JsResult {
    let input: DashboardRequestPayload = serde_wasm_bindgen::from_value(input)?;
    let api_response: ConnectorApiIntegrationPayload = serde_wasm_bindgen::from_value(response)?;
    let result = DashboardRequestPayload::create_connector_request(input, api_response);
    Ok(serde_wasm_bindgen::to_value(&result)?)
}

#[wasm_bindgen(js_name = getResponsePayload)]
/// Takes a JsValue input, deserializes it into a ConnectorApiIntegrationPayload, 
/// calls get_transformed_response_payload on the payload, and then serializes the 
/// result back into a JsValue before returning it as a JsResult.
pub fn get_response_payload(input: JsValue) -> JsResult {
    let input: ConnectorApiIntegrationPayload = serde_wasm_bindgen::from_value(input)?;
    let result = ConnectorApiIntegrationPayload::get_transformed_response_payload(input);
    Ok(serde_wasm_bindgen::to_value(&result)?)
}
