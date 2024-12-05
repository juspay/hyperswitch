use std::collections::{HashMap, HashSet};

use api_models::enums as api_enums;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CountryCurrencyFilter {
    pub connector_configs: HashMap<api_enums::RoutableConnectors, PaymentMethodFilters>,
    pub default_configs: Option<PaymentMethodFilters>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct PaymentMethodFilters(pub HashMap<PaymentMethodFilterKey, CurrencyCountryFlowFilter>);

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum PaymentMethodFilterKey {
    PaymentMethodType(api_enums::PaymentMethodType),
    CardNetwork(api_enums::CardNetwork),
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct CurrencyCountryFlowFilter {
    pub currency: Option<HashSet<api_enums::Currency>>,
    pub country: Option<HashSet<api_enums::CountryAlpha2>>,
    pub not_available_flows: Option<NotAvailableFlows>,
}

#[derive(Debug, Deserialize, Copy, Clone, Default)]
#[serde(default)]
pub struct NotAvailableFlows {
    pub capture_method: Option<api_enums::CaptureMethod>,
}
